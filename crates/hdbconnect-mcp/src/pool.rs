use std::time::Duration;

use deadpool::managed::{self, Metrics, RecycleResult};
use hdbconnect_async::Connection;

pub type Pool = managed::Pool<ConnectionManager>;
pub type PooledConnection = managed::Object<ConnectionManager>;

#[derive(Debug)]
pub struct ConnectionManager {
    url: String,
}

impl ConnectionManager {
    pub const fn new(url: String) -> Self {
        Self { url }
    }
}

impl managed::Manager for ConnectionManager {
    type Type = Connection;
    type Error = hdbconnect::HdbError;

    async fn create(&self) -> Result<Connection, hdbconnect::HdbError> {
        Connection::new(self.url.clone()).await
    }

    async fn recycle(&self, _conn: &mut Connection, _: &Metrics) -> RecycleResult<Self::Error> {
        // Skip connection validation during recycle.
        // Connection errors will be caught on actual query execution.
        Ok(())
    }
}

pub fn create_pool(url: String, max_size: usize) -> Pool {
    Pool::builder(ConnectionManager::new(url))
        .max_size(max_size)
        .wait_timeout(Some(Duration::from_secs(10)))
        .create_timeout(Some(Duration::from_secs(30)))
        .recycle_timeout(Some(Duration::from_secs(5)))
        .build()
        .expect("Failed to create connection pool")
}
