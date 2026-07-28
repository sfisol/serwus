//! Pool made of pools, one writable and others read-only with connections to slave replica(s).

use diesel::r2d2::ConnectionManager;
use log::{error, info};
use r2d2::{Error, PooledConnection};
use serde::Serialize;
use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
};
use weighted_rs::{RoundrobinWeight, Weight};

use crate::threads::num_threads;

use super::{DbConnection, Pool, database_url};

/// Pool made of pools, one writable and others read-only with connections to slave replica(s).
///
/// Example:
///
/// ```
/// use serwus::db_pool::multi::{MultiPool, MultiPoolBuilder};
///
/// pub struct AppData {
///    db_pool: MultiPool,
/// }
///
/// impl Default for AppData {
///    fn default() -> Self {
///       Self {
///          // Use DATABASE_URL env for writable database
///          // Use DATABASE_MIRRORS_URLS env for read-only databases (comma-separated)
///          db_pool: MultiPoolBuilder::default()
///             .connect()
///             .expect("Can't connect to databases")
///       }
///    }
/// }
/// ```
#[derive(Clone)]
pub struct MultiPool {
    master: Option<Pool>,
    mirrors: Vec<Pool>,
    dispatcher: Arc<Mutex<RoundrobinWeight<usize>>>,
}

#[derive(Debug)]
pub enum InitMultiError {
    MasterFailed(Error),
    MirrorFailed((String, Error)),
}

/// Builder for [`MultiPool`].
///
/// Pool sizing and the r2d2 tuning knobs are optional; anything left unset keeps r2d2's own
/// default.
///
/// ```no_run
/// use std::time::Duration;
/// use serwus::db_pool::multi::MultiPoolBuilder;
///
/// let pool = MultiPoolBuilder::default()
///     .size(16)
///     .connection_timeout(Duration::from_secs(3))
///     .min_idle(Some(2))
///     .connect()
///     .expect("Can't connect to databases");
/// ```
pub struct MultiPoolBuilder<'a> {
    size: usize,
    write_url_env: &'a str,
    read_url_env: &'a str,
    read_only: bool,
    connection_timeout: Option<Duration>,
    min_idle: Option<Option<u32>>,
    idle_timeout: Option<Option<Duration>>,
    max_lifetime: Option<Option<Duration>>,
    test_on_check_out: Option<bool>,
}

impl Default for MultiPoolBuilder<'_> {
    fn default() -> Self {
        let nthreads = num_threads();
        Self {
            size: if nthreads > 1 { nthreads } else { 2 },
            write_url_env: "DATABASE_URL",
            read_url_env: "DATABASE_MIRRORS_URLS",
            read_only: false,
            connection_timeout: None,
            min_idle: None,
            idle_timeout: None,
            max_lifetime: None,
            test_on_check_out: None,
        }
    }
}

impl<'a> MultiPoolBuilder<'a> {
    #[must_use]
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn write_url_env(mut self, write_url_env: &'a str) -> Self {
        self.write_url_env = write_url_env;
        self
    }

    #[must_use]
    pub fn read_url_env(mut self, read_url_env: &'a str) -> Self {
        self.read_url_env = read_url_env;
        self
    }

    #[must_use]
    pub fn readonly(mut self) -> Self {
        self.read_only = true;
        self
    }

    /// How long `MultiPool::read`/`write` wait for a free connection before returning
    /// `r2d2::Error`. Also bounds how long [`Self::connect`] waits for the pool to establish its
    /// initial connections, so keep it comfortably above the time it takes to reach the database.
    ///
    /// Leave unset for r2d2's default of 30 seconds. A shorter value makes an overloaded pool fail
    /// fast instead of queueing requests behind it.
    #[must_use]
    pub fn connection_timeout(mut self, connection_timeout: Duration) -> Self {
        self.connection_timeout = Some(connection_timeout);
        self
    }

    /// Minimum number of idle connections each pool keeps warm. `None` means "as many as
    /// `size`". Leave unset for r2d2's default (`None`).
    ///
    /// Clamped to the effective pool size on [`Self::connect`], since `size` itself is capped
    /// when the `TEST` env variable is present — r2d2 panics on `min_idle > max_size`.
    #[must_use]
    pub fn min_idle(mut self, min_idle: Option<u32>) -> Self {
        self.min_idle = Some(min_idle);
        self
    }

    /// How long a connection may sit idle before being closed. `None` disables the timeout.
    /// Leave unset for r2d2's default of 10 minutes.
    #[must_use]
    pub fn idle_timeout(mut self, idle_timeout: Option<Duration>) -> Self {
        self.idle_timeout = Some(idle_timeout);
        self
    }

    /// Maximum lifetime of a connection before it is recycled. `None` disables recycling.
    /// Leave unset for r2d2's default of 30 minutes.
    #[must_use]
    pub fn max_lifetime(mut self, max_lifetime: Option<Duration>) -> Self {
        self.max_lifetime = Some(max_lifetime);
        self
    }

    /// Whether each connection is health-checked before being handed out. Leave unset for r2d2's
    /// default of `true`. Disabling it removes a database round-trip per checkout at the cost of
    /// occasionally handing out a connection the server has already dropped.
    #[must_use]
    pub fn test_on_check_out(mut self, test_on_check_out: bool) -> Self {
        self.test_on_check_out = Some(test_on_check_out);
        self
    }

    /// Applies the configured options to a fresh r2d2 builder. Options left unset keep r2d2's
    /// own defaults, so callers that don't touch them are unaffected.
    #[allow(clippy::cast_possible_truncation)]
    fn pool_builder(&self, max_size: usize) -> r2d2::Builder<ConnectionManager<DbConnection>> {
        let mut builder = Pool::builder().max_size(max_size as u32);

        if let Some(connection_timeout) = self.connection_timeout {
            builder = builder.connection_timeout(connection_timeout);
        }
        if let Some(min_idle) = self.min_idle {
            // r2d2 panics on min_idle > max_size, and max_size may have been capped below the
            // requested size (see `connect`), so clamp rather than let that surface as a panic.
            builder = builder.min_idle(min_idle.map(|min_idle| {
                if min_idle as usize > max_size {
                    info!("Capping min_idle {min_idle} to pool size {max_size}");
                    max_size as u32
                } else {
                    min_idle
                }
            }));
        }
        if let Some(idle_timeout) = self.idle_timeout {
            builder = builder.idle_timeout(idle_timeout);
        }
        if let Some(max_lifetime) = self.max_lifetime {
            builder = builder.max_lifetime(max_lifetime);
        }
        if let Some(test_on_check_out) = self.test_on_check_out {
            builder = builder.test_on_check_out(test_on_check_out);
        }

        builder
    }

    pub fn connect(self) -> Result<MultiPool, InitMultiError> {
        info!("Connecting to database(s)");

        let max_size = if env::var("TEST").is_ok() && self.size > 2 {
            2
        } else {
            self.size
        };

        let master = if self.read_only {
            None
        } else {
            let manager = ConnectionManager::<DbConnection>::new(database_url(self.write_url_env));

            Some(self.pool_builder(max_size).build(manager).map_err(|err| {
                error!("Can't connect to database: {}", err);
                InitMultiError::MasterFailed(err)
            })?)
        };

        let mut mirrors = vec![];
        let mut dispatcher = RoundrobinWeight::new();

        for url in database_mirrors_urls(self.read_url_env) {
            let manager = ConnectionManager::<DbConnection>::new(url.clone());

            mirrors.push(self.pool_builder(max_size).build(manager).map_err(|err| {
                error!("Can't connect to database: {}", err);
                InitMultiError::MirrorFailed((url, err))
            })?);

            dispatcher.add(mirrors.len() - 1, 1);
        }

        if self.read_only {
            info!(
                "Initialized read only pool with {} nodes with {} conns each",
                mirrors.len(),
                max_size
            );
        } else {
            info!(
                "Initialized writable pool with {} read mirror(s) with {} conns each",
                mirrors.len(),
                max_size
            );
        }

        Ok(MultiPool {
            master,
            mirrors,
            dispatcher: Arc::new(Mutex::new(dispatcher)),
        })
    }
}

impl MultiPool {
    pub fn write(&self) -> Result<PooledConnection<ConnectionManager<DbConnection>>, Error> {
        self.master.as_ref().expect("Readonly database pool").get()
    }

    pub fn read(&self) -> Result<PooledConnection<ConnectionManager<DbConnection>>, Error> {
        let n_opt = match self.dispatcher.lock() {
            Ok(mut dispatcher) => dispatcher.next(),
            Err(_) => {
                error!("Error acquiring mirrors mutex, returning master db connection");
                None
            }
        };

        if let Some(n) = n_opt {
            self.mirrors[n].get()
        } else {
            self.write()
        }
    }

    pub fn state(&self) -> MultiPoolState {
        let (rw_conns, rw_conns_idle) = self
            .master
            .as_ref()
            .map(|pool| {
                let state = pool.state();
                (state.connections, state.idle_connections)
            })
            .unwrap_or_default();

        let (ro_conns, ro_conns_idle) = self.mirrors.iter().fold((0, 0), |mut agg, pool| {
            let state = pool.state();
            agg.0 += state.connections;
            agg.1 += state.idle_connections;
            agg
        });

        MultiPoolState {
            rw_conns,
            rw_conns_idle,
            ro_conns,
            ro_conns_idle,
        }
    }
}

#[derive(Debug, Default, PartialEq, Serialize)]
pub struct MultiPoolState {
    pub rw_conns: u32,
    pub rw_conns_idle: u32,
    pub ro_conns: u32,
    pub ro_conns_idle: u32,
}

fn database_mirrors_urls(env_name: &str) -> Vec<String> {
    env::var(env_name)
        .map(|value| value.split(',').map(String::from).collect())
        .unwrap_or_default()
}
