use std::num::{NonZeroU32, NonZeroUsize};
use std::time::Duration;

use url::Url;

#[derive(Debug, Clone)]
pub struct Config {
    pub connection_url: Url,
    pub pool_size: NonZeroUsize,
    pub read_only: bool,
    pub row_limit: Option<NonZeroU32>,
    pub query_timeout: Duration,
}

impl Config {
    #[must_use]
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    #[must_use]
    pub const fn row_limit(&self) -> Option<NonZeroU32> {
        self.row_limit
    }
}

#[derive(Debug)]
pub struct ConfigBuilder {
    connection_url: Option<Url>,
    pool_size: NonZeroUsize,
    read_only: bool,
    row_limit: Option<NonZeroU32>,
    query_timeout: Duration,
}

impl ConfigBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            connection_url: None,
            pool_size: NonZeroUsize::new(4).unwrap(),
            read_only: true,
            row_limit: NonZeroU32::new(10000),
            query_timeout: Duration::from_secs(30),
        }
    }

    #[must_use]
    pub fn connection_url(mut self, url: Url) -> Self {
        self.connection_url = Some(url);
        self
    }

    #[must_use]
    pub const fn pool_size(mut self, size: NonZeroUsize) -> Self {
        self.pool_size = size;
        self
    }

    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub const fn row_limit(mut self, limit: Option<NonZeroU32>) -> Self {
        self.row_limit = limit;
        self
    }

    #[must_use]
    pub const fn query_timeout(mut self, timeout: Duration) -> Self {
        self.query_timeout = timeout;
        self
    }

    pub fn build(self) -> crate::Result<Config> {
        let connection_url = self
            .connection_url
            .ok_or_else(|| crate::Error::Config("connection_url is required".into()))?;

        Ok(Config {
            connection_url,
            pool_size: self.pool_size,
            read_only: self.read_only,
            row_limit: self.row_limit,
            query_timeout: self.query_timeout,
        })
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
