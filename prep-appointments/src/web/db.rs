//! Async Postgres connection pool used by the persistence layer.

use deadpool_postgres::{Config as PgConfig, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::error::Error;
use std::str::FromStr;
use tokio_postgres::config::Config as TpgConfig;
use tokio_postgres::NoTls;

#[derive(Clone)]
pub struct PgPool {
    inner: Pool,
}

impl PgPool {
    /// Build a pool from `DATABASE_URL`. The URL is parsed via `tokio_postgres::Config`
    /// so we accept the same `postgres://user:pass@host:port/db` form as before.
    pub fn from_env() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL is required when STORAGE_BACKEND=postgres")?;
        Self::from_url(&url)
    }

    pub fn from_url(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let tpg = TpgConfig::from_str(database_url)?;

        let mut cfg = PgConfig::new();
        cfg.dbname = tpg.get_dbname().map(|s| s.to_string());
        cfg.user = tpg.get_user().map(|s| s.to_string());
        cfg.password = tpg
            .get_password()
            .map(|p| String::from_utf8_lossy(p).into_owned());
        if let Some(host) = tpg.get_hosts().first() {
            cfg.host = Some(match host {
                tokio_postgres::config::Host::Tcp(h) => h.clone(),
                #[cfg(unix)]
                tokio_postgres::config::Host::Unix(p) => p.to_string_lossy().into_owned(),
            });
        }
        if let Some(port) = tpg.get_ports().first() {
            cfg.port = Some(*port);
        }
        cfg.application_name = Some("kingshot-backend".to_string());
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        Ok(Self { inner: pool })
    }

    pub async fn client(&self) -> Result<deadpool_postgres::Object, Box<dyn Error + Send + Sync>> {
        self.inner
            .get()
            .await
            .map_err(|e| Box::<dyn Error + Send + Sync>::from(e.to_string()))
    }
}
