use std::{
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Context;
use database::Database;
use directories::ProjectDirs;

use crate::database::ScriptInfo;

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

pub fn print_established_trees(db: &Database) -> anyhow::Result<()> {
    let roots = db.get_tree_roots()?;
    if !roots.is_empty() {
        eprintln!("The following trees are established:");
        for path in roots {
            eprintln!("{}", path.display());
        }
    }
    eprintln!();
    Ok(())
}

pub fn checkout(name: &str, ctx: &mut AppContext) -> anyhow::Result<()> {
    let script = ctx.db.get_script_by_name(ctx.root_id, name)?;
    std::fs::write(format!("{}.{}", name, script_ext()), script)?;
    Ok(())
}

pub fn cat(name: &str, ctx: &mut AppContext) -> anyhow::Result<()> {
    let script = ctx.db.get_script_by_name(ctx.root_id, name)?;
    std::io::stdout().write_all(&script)?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn script_ext() -> &'static str {
    "sh"
}

#[cfg(target_os = "windows")]
fn script_ext() -> &'static str {
    "bat"
}

pub fn list_scripts(ctx: &mut AppContext) -> anyhow::Result<()> {
    let scripts = ctx.db.scripts_for_tree(ctx.root_id)?;
    if scripts.is_empty() {
        eprintln!("No scripts have been added yet. To add one, use otkeep add.");
    } else {
        eprintln!("The following scripts are available:\n");
        for ScriptInfo { name, description } in scripts {
            eprintln!(
                "{}{}{}",
                name,
                if description.is_empty() { "" } else { " - " },
                description
            );
        }
    }
    Ok(())
}
