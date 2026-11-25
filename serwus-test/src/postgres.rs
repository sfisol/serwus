//! Postgres in a container, and a client for setting up data and checking what the service
//! wrote.

use std::{
    path::Path,
    sync::atomic::{AtomicU32, Ordering},
};

use anyhow::{Context, Result};
use testcontainers_modules::{
    postgres::Postgres as PostgresImage,
    testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner},
};
use tokio::sync::Mutex;
use tokio_postgres::{
    Client, NoTls, Row,
    types::{FromSqlOwned, ToSql},
};

/// The database a suite gets.
#[derive(Debug, Clone, Copy)]
pub struct PgSpec<'a> {
    pub db: &'a str,
    pub user: &'a str,
    pub password: &'a str,
    /// `name:tag`, unless `E2E_PG_IMAGE` gives another one.
    pub image: &'a str,
}

pub struct Postgres {
    /// `postgres://user:password@<host>:<port>/db`. `<host>` is `127.0.0.1` with a local Docker,
    /// the gateway of the `bridge` network when the tests run in a container next to the Docker
    /// socket (a CI job), and the daemon's host with a TCP `DOCKER_HOST`.
    pub url: String,
    container: Mutex<Option<ContainerAsync<PostgresImage>>>,
}

impl Postgres {
    pub async fn start(spec: PgSpec<'_>, suite: &str) -> Result<Self> {
        let image = std::env::var("E2E_PG_IMAGE").unwrap_or_else(|_| spec.image.to_string());
        let (name, tag) = image.split_once(':').unwrap_or((image.as_str(), "latest"));
        println!("Starting {name}:{tag}");

        let container = PostgresImage::default()
            .with_db_name(spec.db)
            .with_user(spec.user)
            .with_password(spec.password)
            .with_name(name)
            .with_tag(tag)
            .with_label("serwus-test", format!("{}/{suite}", spec.db))
            .start()
            .await
            .context("can't start the Postgres container (is Docker running?)")?;
        let port = container.get_host_port_ipv4(5432).await?;
        let host = container.get_host().await?.to_string();

        Ok(Self {
            url: format!(
                "postgres://{}:{}@{}:{port}/{}",
                spec.user,
                spec.password,
                reachable(&host),
                spec.db
            ),
            container: Mutex::new(Some(container)),
        })
    }

    /// Removes the container.
    pub async fn stop(&self) {
        if let Some(container) = self.container.lock().await.take()
            && let Err(err) = container.rm().await
        {
            eprintln!("can't remove the Postgres container: {err}");
        }
    }
}

/// Where the tests reach the port Docker published, from where testcontainers says the container
/// is: `localhost`, a local Docker, becomes `127.0.0.1`, because the port is the one published on
/// IPv4 and `localhost` may resolve to `::1` first.
fn reachable(host: &str) -> &str {
    if host == "localhost" {
        "127.0.0.1"
    } else {
        host
    }
}

pub struct Db {
    client: Client,
    counter: AtomicU32,
}

impl Db {
    pub async fn connect(url: &str) -> Result<Self> {
        let (client, connection) = tokio_postgres::connect(url, NoTls)
            .await
            .context("can't connect to Postgres")?;
        // Its error at the end of the run, when the container goes away, is noise; a failure
        // in the middle of the run shows up in the queries anyway.
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(Self {
            client,
            counter: AtomicU32::new(0),
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Runs every `<dir>/*/up.sql` in name order, as diesel lays migrations out.
    pub async fn apply_migrations(&self, dir: &Path) -> Result<()> {
        let mut migrations = std::fs::read_dir(dir)
            .with_context(|| format!("can't read {}", dir.display()))?
            .map(|entry| entry.map(|entry| entry.path().join("up.sql")))
            .collect::<std::io::Result<Vec<_>>>()?;
        migrations.retain(|path| path.is_file());
        migrations.sort();

        for path in migrations {
            let sql = std::fs::read_to_string(&path)?;
            self.client
                .batch_execute(&sql)
                .await
                .with_context(|| format!("migration {} failed", path.display()))?;
        }
        Ok(())
    }

    /// Runs `sql`, which may hold many statements. `what` names it in the error.
    pub async fn batch(&self, what: &str, sql: &str) -> Result<()> {
        self.client
            .batch_execute(sql)
            .await
            .with_context(|| format!("{what} failed"))
    }

    pub async fn query(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>> {
        self.client
            .query(sql, params)
            .await
            .with_context(|| format!("query failed: {sql}"))
    }

    pub async fn query_one(&self, sql: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row> {
        self.client
            .query_one(sql, params)
            .await
            .with_context(|| format!("query failed: {sql}"))
    }

    pub async fn scalar<T: FromSqlOwned>(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<T> {
        Ok(self.query_one(sql, params).await?.try_get(0)?)
    }

    pub async fn opt_scalar<T: FromSqlOwned>(
        &self,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<T>> {
        let row = self
            .client
            .query_opt(sql, params)
            .await
            .with_context(|| format!("query failed: {sql}"))?;
        Ok(row.map(|row| row.try_get(0)).transpose()?)
    }

    /// Whether `table` has a row with `id_column = id`.
    pub async fn exists<I: ToSql + Sync>(
        &self,
        table: &str,
        id_column: &str,
        id: I,
    ) -> Result<bool> {
        self.scalar(
            &format!("SELECT EXISTS (SELECT 1 FROM {table} WHERE {id_column} = $1)"),
            &[&id],
        )
        .await
    }

    /// The next number of a sequence shared by the suite, for names of fixtures that must be
    /// unique.
    pub fn next(&self) -> u32 {
        self.counter.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_local_docker_is_reached_on_the_ipv4_loopback() {
        assert_eq!(reachable("localhost"), "127.0.0.1");
    }

    #[test]
    fn a_gateway_or_the_host_of_a_remote_docker_stays() {
        assert_eq!(reachable("172.17.0.1"), "172.17.0.1");
        assert_eq!(reachable("docker"), "docker");
        assert_eq!(reachable("[::1]"), "[::1]");
    }

    /// Run where Docker is, `cargo test -p serwus-test -- --ignored`, also in a container next to
    /// the Docker socket, as a CI job runs.
    #[tokio::test]
    #[ignore = "needs Docker"]
    async fn the_tests_reach_the_database_they_started() -> Result<()> {
        let spec = PgSpec {
            db: "cats",
            user: "cat",
            password: "clearly-not-a-dog",
            image: "postgres:18",
        };
        let postgres = Postgres::start(spec, "reach").await?;
        let answer = async {
            Db::connect(&postgres.url)
                .await?
                .scalar::<String>("SELECT 'meow'", &[])
                .await
        }
        .await;
        postgres.stop().await;
        assert_eq!(answer?, "meow");
        Ok(())
    }
}
