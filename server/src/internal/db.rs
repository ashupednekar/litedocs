use std::sync::Arc;

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::query;
use sqlx::PgPool;

pub async fn connect_if_configured(
    database_url: Option<&str>,
) -> Result<Option<Arc<PgPool>>, sqlx::Error> {
    match database_url {
        Some(url) => {
            let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;
            Ok(Some(Arc::new(pool)))
        }
        None => Ok(None),
    }
}

#[async_trait]
pub trait DbSessionOps {
    async fn ensure_search_path(&self, schema: &str) -> Result<(), sqlx::Error>;
}

#[async_trait]
impl DbSessionOps for Arc<PgPool> {
    async fn ensure_search_path(&self, schema: &str) -> Result<(), sqlx::Error> {
        let mut conn = self.acquire().await?;
        query("SELECT set_config('search_path', $1, false)")
            .bind(schema)
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
