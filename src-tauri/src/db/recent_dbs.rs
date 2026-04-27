use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::db::AppError;

const MAX_RECENT: usize = 10;

/// A single recent database entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecentDb {
    pub path: String,
    pub label: String,
    pub last_opened: String, // ISO 8601
}

/// Manages a list of recently opened database paths in a JSON file.
pub struct RecentDbs {
    config_path: PathBuf,
    entries: Vec<RecentDb>,
}

impl RecentDbs {
    /// Create an empty recent-dbs list (no persistence).
    pub fn empty() -> Self {
        Self {
            config_path: PathBuf::new(),
            entries: Vec::new(),
        }
    }

    /// Load recent databases from the config file.
    pub fn load() -> Result<Self, AppError> {
        let config_path = config_file_path()
            .ok_or_else(|| AppError::IoError("Cannot determine config directory".to_string()))?;

        Self::load_from_path(&config_path)
    }

    /// Load from a specific path (internal, also used by tests).
    pub fn load_from_path(config_path: &std::path::Path) -> Result<Self, AppError> {
        let entries = if config_path.exists() {
            let content = fs::read_to_string(config_path)
                .map_err(|e| AppError::IoError(format!("Failed to read config: {}", e)))?;
            serde_json::from_str(&content)
                .map_err(|e| AppError::IoError(format!("Failed to parse config: {}", e)))?
        } else {
            Vec::new()
        };

        Ok(Self {
            config_path: config_path.to_path_buf(),
            entries,
        })
    }

    /// Return the current list of recent databases.
    pub fn list(&self) -> &[RecentDb] {
        &self.entries
    }

    /// Add or update a recent database entry.
    /// If the path already exists, it moves to the top and updates the timestamp.
    pub fn add(&mut self, path: &str, label: &str) -> Result<(), AppError> {
        // Remove existing entry with same path
        self.entries.retain(|e| e.path != path);

        // Insert at the top
        let entry = RecentDb {
            path: path.to_string(),
            label: label.to_string(),
            last_opened: crate::utils::time::now_rfc3339(),
        };
        self.entries.insert(0, entry);

        // Keep only MAX_RECENT
        if self.entries.len() > MAX_RECENT {
            self.entries.truncate(MAX_RECENT);
        }

        self.save()
    }

    /// Remove a recent database entry by path.
    pub fn remove(&mut self, path: &str) -> Result<(), AppError> {
        self.entries.retain(|e| e.path != path);
        self.save()
    }

    /// Persist the list to the config file.
    fn save(&self) -> Result<(), AppError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::IoError(format!("Failed to create config dir: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| AppError::IoError(format!("Failed to serialize config: {}", e)))?;

        fs::write(&self.config_path, content)
            .map_err(|e| AppError::IoError(format!("Failed to write config: {}", e)))
    }
}

/// Return the path to the recent-dbs JSON config file.
fn config_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("glimmerx").join("recent_dbs.json"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_recent_dbs() -> RecentDbs {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("recent_dbs.json");
        RecentDbs {
            config_path,
            entries: Vec::new(),
        }
    }

    #[test]
    fn test_add_and_list() {
        let mut rd = test_recent_dbs();
        rd.add("/path/a.db", "Database A").unwrap();
        rd.add("/path/b.db", "Database B").unwrap();

        let list = rd.list();
        assert_eq!(list.len(), 2);
        // Most recent first
        assert_eq!(list[0].path, "/path/b.db");
        assert_eq!(list[1].path, "/path/a.db");
    }

    #[test]
    fn test_add_existing_moves_to_top() {
        let mut rd = test_recent_dbs();
        rd.add("/path/a.db", "Database A").unwrap();
        rd.add("/path/b.db", "Database B").unwrap();
        rd.add("/path/a.db", "Database A (renamed)").unwrap();

        let list = rd.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, "/path/a.db");
        assert_eq!(list[0].label, "Database A (renamed)");
    }

    #[test]
    fn test_max_entries() {
        let mut rd = test_recent_dbs();
        for i in 0..15 {
            rd.add(&format!("/path/db_{}.db", i), &format!("DB {}", i))
                .unwrap();
        }
        assert_eq!(rd.list().len(), MAX_RECENT);
        // Most recent should be db_14
        assert_eq!(rd.list()[0].path, "/path/db_14.db");
    }

    #[test]
    fn test_remove() {
        let mut rd = test_recent_dbs();
        rd.add("/path/a.db", "A").unwrap();
        rd.add("/path/b.db", "B").unwrap();
        rd.remove("/path/a.db").unwrap();
        assert_eq!(rd.list().len(), 1);
        assert_eq!(rd.list()[0].path, "/path/b.db");
    }

    #[test]
    fn test_save_and_load() {
        let mut rd = test_recent_dbs();
        rd.add("/path/a.db", "A").unwrap();
        rd.add("/path/b.db", "B").unwrap();

        // Reload from disk
        let config_path = rd.config_path.clone();
        let loaded = RecentDbs::load_from_path(&config_path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
    }
}
