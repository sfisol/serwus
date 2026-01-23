//! Helpers for creating r2d2 pool based on ENV variables and number of CPUs.

mod single;
pub use single::*;

#[cfg(feature = "multidb")]
pub mod multi;

mod async_queries;
pub use async_queries::*;

#[cfg(all(feature = "mysql", not(feature = "pgsql")))]
use diesel::mysql::MysqlConnection;
#[cfg(feature = "pgsql")]
use diesel::pg::PgConnection;

#[cfg(feature = "pgsql")]
pub type DbConnection = PgConnection;
#[cfg(all(feature = "mysql", not(feature = "pgsql")))]
pub type DbConnection = MysqlConnection;

#[cfg(all(feature = "mysql", not(feature = "pgsql")))]
use diesel::mysql::Mysql;
#[cfg(feature = "pgsql")]
use diesel::pg::Pg;

#[cfg(feature = "pgsql")]
pub type Db = Pg;
#[cfg(all(feature = "mysql", not(feature = "pgsql")))]
pub type Db = Mysql;
