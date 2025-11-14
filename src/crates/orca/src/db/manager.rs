///! Database Manager for dual-database architecture
///!
///! Manages connections to both user-level and project-level databases:
///! - User DB: ~/.orca/user.db (LLM configs, prompts, workflow templates)
///! - Project DB: <project>/.orca/project.db (workflows, tasks, bugs, rules, permissions)

use crate::db::Database;
use crate::error::{OrcaError, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Database type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseType {
    /// User-level database (~/.orca/user.db)
    User,
    /// Project-level database (<project>/.orca/project.db)
    Project,
}

/// Manages connections to both user and project databases
#[derive(Clone, Debug)]
pub struct DatabaseManager {
    /// User-level database connection
    user_db: Arc<Database>,

    /// Project-level database connection (optional - may not exist)
    project_db: Option<Arc<Database>>,

    /// Project root directory (where .orca/ lives)
    project_root: Option<PathBuf>,
}

impl DatabaseManager {
    /// Create a new database manager
    ///
    /// # Arguments
    /// * `workspace_root` - Current working directory or project root
    ///
    /// # Returns
    /// DatabaseManager with user DB initialized, project DB lazy-loaded
    pub async fn new(workspace_root: impl AsRef<Path>) -> Result<Self> {
        // Initialize user database
        let user_db_path = Self::get_user_db_path()?;
        debug!("Initializing user database at: {}", user_db_path.display());

        let user_db = Arc::new(Database::initialize(&user_db_path).await?);

        // Run user migrations
        Self::run_migrations(&user_db, DatabaseType::User).await?;

        info!("User database initialized");

        // Detect project root
        let project_root = Self::find_project_root(workspace_root.as_ref());

        // Initialize project database if project root exists
        let project_db = if let Some(ref root) = project_root {
            debug!("Found project root at: {}", root.display());
            match Self::initialize_project_db(root).await {
                Ok(db) => {
                    info!("Project database initialized");
                    Some(Arc::new(db))
                }
                Err(e) => {
                    warn!("Could not initialize project database: {}", e);
                    None
                }
            }
        } else {
            debug!("No project root found, running in user-only mode");
            None
        };

        Ok(Self {
            user_db,
            project_db,
            project_root,
        })
    }

    /// Get user database connection
    pub fn user_db(&self) -> &Arc<Database> {
        &self.user_db
    }

    /// Get project database connection
    ///
    /// Returns None if no project context exists
    pub fn project_db(&self) -> Option<&Arc<Database>> {
        self.project_db.as_ref()
    }

    /// Check if project database is available
    pub fn has_project(&self) -> bool {
        self.project_db.is_some()
    }

    /// Get project root directory
    pub fn project_root(&self) -> Option<&Path> {
        self.project_root.as_deref()
    }

    /// Ensure project database exists, creating it if necessary
    ///
    /// Call this before any project-specific operation
    pub async fn ensure_project_db(&mut self) -> Result<&Arc<Database>> {
        if let Some(ref db) = self.project_db {
            return Ok(db);
        }

        // Need to create project database
        let workspace_root = std::env::current_dir()
            .map_err(|e| OrcaError::Other(format!("Cannot determine current directory: {}", e)))?;

        // Create .orca/ directory in current workspace
        let project_orca_dir = workspace_root.join(".orca");
        std::fs::create_dir_all(&project_orca_dir)
            .map_err(|e| OrcaError::Other(format!("Failed to create .orca directory: {}", e)))?;

        info!("Creating new project database at: {}", project_orca_dir.display());

        // Initialize project database
        let db = Self::initialize_project_db(&workspace_root).await?;
        let db = Arc::new(db);

        self.project_db = Some(db.clone());
        self.project_root = Some(workspace_root);

        Ok(self.project_db.as_ref().unwrap())
    }

    /// Get user database path (~/.orca/user.db)
    fn get_user_db_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| OrcaError::Other("Cannot determine home directory".to_string()))?;

        let orca_dir = home.join(".orca");

        // Ensure directory exists
        std::fs::create_dir_all(&orca_dir)
            .map_err(|e| OrcaError::Other(format!("Failed to create ~/.orca directory: {}", e)))?;

        Ok(orca_dir.join("user.db"))
    }

    /// Get project database path (<project>/.orca/project.db)
    fn get_project_db_path(project_root: &Path) -> PathBuf {
        project_root.join(".orca").join("project.db")
    }

    /// Find project root by searching for .orca directory in parent directories
    ///
    /// Returns None if no project root found
    fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
        let mut current = start_dir;

        loop {
            let orca_dir = current.join(".orca");
            if orca_dir.exists() && orca_dir.is_dir() {
                debug!("Found .orca directory at: {}", current.display());
                return Some(current.to_path_buf());
            }

            // Move to parent directory
            current = current.parent()?;
        }
    }

    /// Initialize project database
    async fn initialize_project_db(project_root: &Path) -> Result<Database> {
        let project_db_path = Self::get_project_db_path(project_root);

        // Ensure .orca directory exists
        let orca_dir = project_root.join(".orca");
        std::fs::create_dir_all(&orca_dir)
            .map_err(|e| OrcaError::Other(format!("Failed to create .orca directory: {}", e)))?;

        // Initialize database
        let db = Database::initialize(&project_db_path).await?;

        // Run project migrations
        Self::run_migrations(&db, DatabaseType::Project).await?;

        Ok(db)
    }

    /// Run migrations for the specified database type
    async fn run_migrations(db: &Database, db_type: DatabaseType) -> Result<()> {
        match db_type {
            DatabaseType::User => {
                // User migrations are in migrations/user/
                db.run_migrations_from("migrations/user").await?;
            }
            DatabaseType::Project => {
                // Project migrations are in migrations/project/
                db.run_migrations_from("migrations/project").await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_database_manager_user_only() {
        let temp_workspace = TempDir::new().unwrap();

        // Create manager without project database
        let manager = DatabaseManager::new(temp_workspace.path()).await.unwrap();

        // Should have user DB
        assert!(!manager.user_db().pool().is_closed());

        // Should not have project DB (no .orca/ in temp dir)
        assert!(!manager.has_project());
        assert!(manager.project_db().is_none());
        assert!(manager.project_root().is_none());
    }

    #[tokio::test]
    async fn test_database_manager_with_project() {
        let temp_workspace = TempDir::new().unwrap();

        // Create .orca directory to simulate project
        std::fs::create_dir_all(temp_workspace.path().join(".orca")).unwrap();

        // Create manager
        let manager = DatabaseManager::new(temp_workspace.path()).await.unwrap();

        // Should have user DB
        assert!(!manager.user_db().pool().is_closed());

        // Should have project DB
        assert!(manager.has_project());
        assert!(manager.project_db().is_some());
        assert_eq!(manager.project_root(), Some(temp_workspace.path()));
    }

    #[tokio::test]
    async fn test_ensure_project_db_creates_if_missing() {
        let temp_workspace = TempDir::new().unwrap();
        std::env::set_current_dir(temp_workspace.path()).unwrap();

        // Create manager without project
        let mut manager = DatabaseManager::new(temp_workspace.path()).await.unwrap();
        assert!(!manager.has_project());

        // Ensure project DB - should create it
        let project_db = manager.ensure_project_db().await.unwrap();
        assert!(!project_db.pool().is_closed());

        // Now should have project
        assert!(manager.has_project());
        assert!(manager.project_db().is_some());

        // .orca directory should exist
        assert!(temp_workspace.path().join(".orca").exists());
    }

    #[test]
    fn test_find_project_root() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let subdir = project_root.join("src").join("deeply").join("nested");

        // Create .orca at project root
        std::fs::create_dir_all(project_root.join(".orca")).unwrap();
        std::fs::create_dir_all(&subdir).unwrap();

        // Should find project root from nested directory
        let found = DatabaseManager::find_project_root(&subdir);
        assert_eq!(found, Some(project_root.to_path_buf()));
    }

    #[test]
    fn test_find_project_root_no_project() {
        let temp_dir = TempDir::new().unwrap();

        // No .orca directory exists
        let found = DatabaseManager::find_project_root(temp_dir.path());
        assert_eq!(found, None);
    }
}
