use std::path::{Path, PathBuf};

use anyhow::Context;
use database::Database;
use directories::ProjectDirs;

pub mod database;
mod fs_util;
mod run;

/// Contains the settings and the script database.
pub struct AppContext {
    pub db: Database,
    pub root_id: i64,
}

pub fn load_db() -> anyhow::Result<Database> {
    let dirs =
        ProjectDirs::from("", "crumblingstatue", "otkeep").context("Failed to get project dirs")?;
    let data_dir = dirs.data_dir();
    let db = Database::load(data_dir)?;
    Ok(db)
}

pub fn find_root(database: &Database) -> anyhow::Result<Option<(i64, PathBuf)>> {
    let current_dir = std::env::current_dir()?;
    let mut opt_path: Option<&Path> = Some(&current_dir);
    while let Some(path) = opt_path {
        match database.query_tree(path)? {
            Some(id) => return Ok(Some((id, path.to_owned()))),
            None => {
                opt_path = path.parent();
            }
        }
    }
    Ok(None)
}
