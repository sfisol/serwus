use log::{info};

#[cfg(feature = "swagger")]
use paperclip::actix::{web, OpenApiExt};

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
    db_connection: bool,
}

impl StatsPresenter<DefaultServiceStats> for DefaultAppData {
    fn get_stats(&self) -> Box<dyn Future<Item=DefaultServiceStats, Error=Error>> {
        let db_connection = self.db_pool.get().is_ok();

        let fut = fut_ok(
            DefaultServiceStats {
                db_connection,
            }
        );

        Box::new(fut)
    }
}
