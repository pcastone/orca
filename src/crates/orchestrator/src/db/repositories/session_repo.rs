//! Session repository for database operations

use crate::db::connection::DatabasePool;
use crate::db::models::Session;
use chrono::Utc;

/// Session repository for managing session database operations
pub struct SessionRepository;

impl SessionRepository {
    /// Create a new session
    pub async fn create(
        pool: &DatabasePool,
        id: String,
        client_id: String,
        connection_type: String,
    ) -> Result<Session, sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query_as::<_, Session>(
            "INSERT INTO sessions (id, client_id, connection_type, is_active, created_at, updated_at, last_heartbeat)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING *"
        )
        .bind(&id)
        .bind(&client_id)
        .bind(&connection_type)
        .bind(1)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .fetch_one(pool)
        .await
    }

    /// Get a session by ID
    pub async fn get_by_id(
        pool: &DatabasePool,
        id: &str,
    ) -> Result<Option<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Get a session by client ID
    pub async fn get_by_client_id(
        pool: &DatabasePool,
        client_id: &str,
    ) -> Result<Option<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE client_id = ?")
            .bind(client_id)
            .fetch_optional(pool)
            .await
    }

    /// List all active sessions
    pub async fn list_active(pool: &DatabasePool) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE is_active = 1 ORDER BY last_heartbeat DESC"
        )
        .fetch_all(pool)
        .await
    }

    /// List all inactive sessions
    pub async fn list_inactive(pool: &DatabasePool) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE is_active = 0 ORDER BY updated_at DESC"
        )
        .fetch_all(pool)
        .await
    }

    /// List sessions by user
    pub async fn list_by_user(
        pool: &DatabasePool,
        user_id: &str,
    ) -> Result<Vec<Session>, sqlx::Error> {
        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE user_id = ? ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }

    /// Update session heartbeat
    pub async fn update_heartbeat(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE sessions SET last_heartbeat = ?, updated_at = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Mark session as active
    pub async fn mark_active(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET is_active = ?, updated_at = ? WHERE id = ?")
            .bind(1)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Mark session as inactive
    pub async fn mark_inactive(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET is_active = ?, updated_at = ? WHERE id = ?")
            .bind(0)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update session user ID
    pub async fn update_user_id(
        pool: &DatabasePool,
        id: &str,
        user_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET user_id = ?, updated_at = ? WHERE id = ?")
            .bind(user_id)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Update session metadata
    pub async fn update_metadata(
        pool: &DatabasePool,
        id: &str,
        metadata: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET metadata = ?, updated_at = ? WHERE id = ?")
            .bind(metadata)
            .bind(&now)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete a session
    pub async fn delete(pool: &DatabasePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete all inactive sessions
    pub async fn delete_inactive(pool: &DatabasePool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM sessions WHERE is_active = 0")
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Count total sessions
    pub async fn count(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// Count active sessions
    pub async fn count_active(pool: &DatabasePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE is_active = 1")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }

    /// List sessions with stale heartbeat (older than duration_secs)
    pub async fn list_stale(
        pool: &DatabasePool,
        duration_secs: i64,
    ) -> Result<Vec<Session>, sqlx::Error> {
        let cutoff = Utc::now()
            .checked_sub_signed(chrono::Duration::seconds(duration_secs))
            .unwrap()
            .to_rfc3339();

        sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE is_active = 1 AND last_heartbeat < ? ORDER BY last_heartbeat ASC"
        )
        .bind(&cutoff)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> sqlx::sqlite::SqlitePool {
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE sessions (
                id TEXT PRIMARY KEY NOT NULL,
                client_id TEXT NOT NULL,
                user_id TEXT,
                connection_type TEXT NOT NULL DEFAULT 'websocket',
                is_active INTEGER NOT NULL DEFAULT 1,
                metadata TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_heartbeat TEXT NOT NULL,
                CHECK (is_active IN (0, 1)),
                CHECK (connection_type IN ('websocket', 'http', 'grpc'))
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_create_session() {
        let pool = setup_db().await;

        let session = SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        assert_eq!(session.id, "session-1");
        assert_eq!(session.client_id, "client-1");
        assert_eq!(session.is_active, 1);
    }

    #[tokio::test]
    async fn test_get_by_id() {
        let pool = setup_db().await;

        SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        let session = SessionRepository::get_by_id(&pool, "session-1")
            .await
            .unwrap();

        assert!(session.is_some());
        assert_eq!(session.unwrap().client_id, "client-1");
    }

    #[tokio::test]
    async fn test_list_active() {
        let pool = setup_db().await;

        SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        SessionRepository::create(
            &pool,
            "session-2".to_string(),
            "client-2".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        SessionRepository::mark_inactive(&pool, "session-2")
            .await
            .unwrap();

        let active = SessionRepository::list_active(&pool).await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "session-1");
    }

    #[tokio::test]
    async fn test_mark_inactive() {
        let pool = setup_db().await;

        SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        SessionRepository::mark_inactive(&pool, "session-1")
            .await
            .unwrap();

        let session = SessionRepository::get_by_id(&pool, "session-1")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(session.is_active, 0);
    }

    #[tokio::test]
    async fn test_update_heartbeat() {
        let pool = setup_db().await;

        SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        SessionRepository::update_heartbeat(&pool, "session-1")
            .await
            .unwrap();

        let session = SessionRepository::get_by_id(&pool, "session-1")
            .await
            .unwrap()
            .unwrap();

        assert!(session.last_heartbeat > session.created_at);
    }

    #[tokio::test]
    async fn test_count_active() {
        let pool = setup_db().await;

        SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        SessionRepository::create(
            &pool,
            "session-2".to_string(),
            "client-2".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        let count = SessionRepository::count_active(&pool).await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_delete_session() {
        let pool = setup_db().await;

        SessionRepository::create(
            &pool,
            "session-1".to_string(),
            "client-1".to_string(),
            "websocket".to_string(),
        )
        .await
        .unwrap();

        SessionRepository::delete(&pool, "session-1")
            .await
            .unwrap();

        let session = SessionRepository::get_by_id(&pool, "session-1")
            .await
            .unwrap();

        assert!(session.is_none());
    }
}
