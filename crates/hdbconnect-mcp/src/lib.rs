//! MCP server for SAP HANA database

#[cfg(feature = "cache")]
pub mod cache;
pub mod config;
mod constants;
mod error;
mod helpers;
pub mod observability;
mod pool;
pub mod security;
pub mod server;
pub mod transport;
pub mod types;
mod validation;

#[cfg(feature = "cache")]
pub use cache::{
    CacheBackend, CacheConfig, CacheError, CacheKey, CacheNamespace, CacheProvider, CacheResult,
    CacheStats, CacheTtlConfig, InMemoryCache, NoopCache, TracedCache, create_cache,
};
pub use config::{
    AllowedOperations, Config, ConfigBuilder, DmlConfig, DmlOperation, ProcedureConfig,
    TelemetryConfig, TransportConfig, TransportMode,
};
pub use error::{Error, Result};
pub use pool::{Pool, PooledConnection, create_pool};
pub use security::{QueryGuard, SchemaFilter};
pub use server::ServerHandler;
pub use types::*;
