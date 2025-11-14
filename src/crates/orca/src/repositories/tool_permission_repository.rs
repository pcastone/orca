//! Tool permission repository for database operations

use crate::db::Database;
use crate::error::{OrcaError, Result};
use crate::models::ToolPermission;
use chrono::Utc;
use sqlx::Row;
use std::sync::Arc;

/// Repository for tool permission database operations (project DB)
#[derive(Clone, Debug)]
pub struct ToolPermissionRepository {
    db: Arc<Database>,
}

impl ToolPermissionRepository {
    /// Create a new tool permission repository
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Save a tool permission to the database
    pub async fn save(&self, permission: &ToolPermission) -> Result<()> {
        sqlx::query(
            "INSERT INTO tool_permissions (id, tool_name, permission_level, path_restrictions,
                                          arg_whitelist, arg_blacklist, description,
                                          created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&permission.id)
        .bind(&permission.tool_name)
        .bind(&permission.permission_level)
        .bind(&permission.path_restrictions)
        .bind(&permission.arg_whitelist)
        .bind(&permission.arg_blacklist)
        .bind(&permission.description)
        .bind(permission.created_at)
        .bind(permission.updated_at)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to save tool permission: {}", e)))?;

        Ok(())
    }

    /// Load a tool permission by ID
    pub async fn find_by_id(&self, id: &str) -> Result<ToolPermission> {
        let row = sqlx::query(
            "SELECT id, tool_name, permission_level, path_restrictions, arg_whitelist,
                    arg_blacklist, description, created_at, updated_at
             FROM tool_permissions WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load tool permission: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Tool permission not found: {}", id)))?;

        Ok(ToolPermission {
            id: row.get("id"),
            tool_name: row.get("tool_name"),
            permission_level: row.get("permission_level"),
            path_restrictions: row.get("path_restrictions"),
            arg_whitelist: row.get("arg_whitelist"),
            arg_blacklist: row.get("arg_blacklist"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Find a tool permission by tool name
    pub async fn find_by_tool_name(&self, tool_name: &str) -> Result<ToolPermission> {
        let row = sqlx::query(
            "SELECT id, tool_name, permission_level, path_restrictions, arg_whitelist,
                    arg_blacklist, description, created_at, updated_at
             FROM tool_permissions WHERE tool_name = ?"
        )
        .bind(tool_name)
        .fetch_optional(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to load tool permission: {}", e)))?
        .ok_or_else(|| OrcaError::Database(format!("Tool permission not found for: {}", tool_name)))?;

        Ok(ToolPermission {
            id: row.get("id"),
            tool_name: row.get("tool_name"),
            permission_level: row.get("permission_level"),
            path_restrictions: row.get("path_restrictions"),
            arg_whitelist: row.get("arg_whitelist"),
            arg_blacklist: row.get("arg_blacklist"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// List all tool permissions
    pub async fn list(&self) -> Result<Vec<ToolPermission>> {
        let rows = sqlx::query(
            "SELECT id, tool_name, permission_level, path_restrictions, arg_whitelist,
                    arg_blacklist, description, created_at, updated_at
             FROM tool_permissions
             ORDER BY tool_name ASC"
        )
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list tool permissions: {}", e)))?;

        let permissions = rows
            .into_iter()
            .map(|row| ToolPermission {
                id: row.get("id"),
                tool_name: row.get("tool_name"),
                permission_level: row.get("permission_level"),
                path_restrictions: row.get("path_restrictions"),
                arg_whitelist: row.get("arg_whitelist"),
                arg_blacklist: row.get("arg_blacklist"),
                description: row.get("description"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(permissions)
    }

    /// List permissions by permission level
    pub async fn list_by_level(&self, level: &str) -> Result<Vec<ToolPermission>> {
        let rows = sqlx::query(
            "SELECT id, tool_name, permission_level, path_restrictions, arg_whitelist,
                    arg_blacklist, description, created_at, updated_at
             FROM tool_permissions
             WHERE permission_level = ?
             ORDER BY tool_name ASC"
        )
        .bind(level)
        .fetch_all(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to list tool permissions by level: {}", e)))?;

        let permissions = rows
            .into_iter()
            .map(|row| ToolPermission {
                id: row.get("id"),
                tool_name: row.get("tool_name"),
                permission_level: row.get("permission_level"),
                path_restrictions: row.get("path_restrictions"),
                arg_whitelist: row.get("arg_whitelist"),
                arg_blacklist: row.get("arg_blacklist"),
                description: row.get("description"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(permissions)
    }

    /// Update a tool permission
    pub async fn update(&self, permission: &ToolPermission) -> Result<()> {
        let updated_at = Utc::now().timestamp();

        sqlx::query(
            "UPDATE tool_permissions
             SET tool_name = ?, permission_level = ?, path_restrictions = ?,
                 arg_whitelist = ?, arg_blacklist = ?, description = ?, updated_at = ?
             WHERE id = ?"
        )
        .bind(&permission.tool_name)
        .bind(&permission.permission_level)
        .bind(&permission.path_restrictions)
        .bind(&permission.arg_whitelist)
        .bind(&permission.arg_blacklist)
        .bind(&permission.description)
        .bind(updated_at)
        .bind(&permission.id)
        .execute(self.db.pool())
        .await
        .map_err(|e| OrcaError::Database(format!("Failed to update tool permission: {}", e)))?;

        Ok(())
    }

    /// Delete a tool permission
    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tool_permissions WHERE id = ?")
            .bind(id)
            .execute(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to delete tool permission: {}", e)))?;

        Ok(())
    }

    /// Check if a tool has permission configured
    pub async fn tool_has_permission(&self, tool_name: &str) -> Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM tool_permissions WHERE tool_name = ?")
            .bind(tool_name)
            .fetch_one(self.db.pool())
            .await
            .map_err(|e| OrcaError::Database(format!("Failed to check tool permission: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count > 0)
    }
}
