use ::diesel::pg::PgConnection;
use ::r2d2::{self, Error};
use ::r2d2_diesel::ConnectionManager;
use ::std::env;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool(size: usize) -> Result<Pool, Error> {
    let manager = ConnectionManager::<PgConnection>::new(database_url());

    let max_size = if env::var("TEST").is_ok() && size > 2 {
        2
    } else {
        size
    };

    #[allow(clippy::cast_possible_truncation)]
    Pool::builder()
        .max_size(max_size as u32)
        .build(manager)
        .map_err(|err| {
            println!("Can't connect to database: {}", err);
            err
        })
}

fn database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}
