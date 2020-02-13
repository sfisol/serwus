#[cfg(feature = "pgsql")]
use log::{info};

use actix_web::Error;
use futures::future::{Future, ok as fut_ok};
use serde::Serialize;

#[cfg(feature = "pgsql")]
use super::db_pool;

use super::stats::StatsPresenter;

#[derive(Clone)]
pub struct DefaultAppData {
    #[cfg(feature = "pgsql")]
    pub db_pool: db_pool::Pool,
}

#[cfg(feature = "pgsql")]
pub fn default_app_data() -> DefaultAppData {
    // FIXME: Create Pool as Actix Actor
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
    fn is_ready(&self) -> Box<dyn Future<Item=bool, Error=Error>> {
        #[cfg(feature = "pgsql")]
        return Box::new(fut_ok(self.db_pool.get().is_ok()));

        #[cfg(not(feature = "pgsql"))]
        return Box::new(fut_ok(true));
    }

    fn get_stats(&self) -> Box<dyn Future<Item=DefaultServiceStats, Error=Error>> {
        #[cfg(feature = "pgsql")]
        let db_connection = self.db_pool.get().is_ok();

        let fut = fut_ok(
            DefaultServiceStats {
                #[cfg(feature = "pgsql")]
                db_connection,
            }
        );

        Box::new(fut)
    }
}
