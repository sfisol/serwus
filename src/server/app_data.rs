#[cfg(feature = "pgsql")]
use log::{info};

use actix_web::Error;
use futures::future::{Future, ok as fut_ok};
use serde::Serialize;
use std::pin::Pin;

#[cfg(feature = "pgsql")]
use super::db_pool;

#[cfg(feature = "prometheus")]
use super::prometheus::ToPrometheus;

use super::stats::StatsPresenter;

#[derive(Clone)]
pub struct DefaultAppData {
    #[cfg(feature = "pgsql")]
    pub db_pool: db_pool::Pool,
}

#[cfg(feature = "pgsql")]
pub fn default_app_data() -> DefaultAppData {
    info!("Connecting to database");
    let db_pool = db_pool::init_default_pool().unwrap();

    DefaultAppData { db_pool }
}

#[cfg(not(feature = "pgsql"))]
pub fn default_app_data() -> DefaultAppData {
    DefaultAppData { }
}


#[derive(Serialize)]
pub struct DefaultServiceStats {
    #[cfg(feature = "pgsql")]
    db_connection: bool,
}

impl StatsPresenter<DefaultServiceStats> for DefaultAppData {
    fn is_ready(&self) -> Pin<Box<dyn Future<Output=Result<bool, Error>>>> {
        #[cfg(feature = "pgsql")]
        let res = self.db_pool.get().is_ok();

        #[cfg(not(feature = "pgsql"))]
        let res = false;

        Box::pin(fut_ok(res))
    }

    fn get_stats(&self) -> Pin<Box<dyn Future<Output=Result<DefaultServiceStats, Error>>>> {
        #[cfg(feature = "pgsql")]
        let db_connection = self.db_pool.get().is_ok();

        let fut = fut_ok(
            DefaultServiceStats {
                #[cfg(feature = "pgsql")]
                db_connection,
            }
        );

        Box::pin(fut)
    }
}

#[cfg(feature = "prometheus")]
impl ToPrometheus for DefaultServiceStats {
    fn to_prometheus(&self) -> Vec<String> {
        let mut out = Vec::new();
        #[cfg(feature = "pgsql")]
        out.push(format!("db_connection {}", self.db_connection as i32));
        out
    }
}
