#![feature(never_type)]

use {
    crate::database::ScriptInfo,
    anyhow::Context,
    database::Database,
    directories::ProjectDirs,
    std::{
        io::Write,
        path::{Path, PathBuf},
    },
};

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
    find_root_for_path(database, &current_dir)
}

pub fn find_root_for_path(
    database: &Database,
    path: &Path,
) -> anyhow::Result<Option<(i64, PathBuf)>> {
    let mut opt_path: Option<&Path> = Some(path);
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
        for root in roots {
            eprintln!("{}", root.path.display());
        }
    }
    eprintln!();
    Ok(())
}

pub fn checkout(name: &str, ctx: &mut AppContext) -> anyhow::Result<()> {
    let script = ctx.db.get_script_by_name(ctx.root_id, name)?;
    std::fs::write(name, script)?;
    Ok(())
}

pub fn cat(name: &str, ctx: &mut AppContext) -> anyhow::Result<()> {
    let script = ctx
        .db
        .get_script_by_name(ctx.root_id, name)
        .or_else(|_| ctx.db.get_file_by_name(ctx.root_id, name))?;
    std::io::stdout().write_all(&script)?;
    Ok(())
}

pub fn rename_script(old_name: &str, new_name: &str, ctx: &mut AppContext) -> anyhow::Result<()> {
    ctx.db.rename_script(old_name, new_name)
}

pub fn list_scripts(ctx: &AppContext) -> anyhow::Result<()> {
    list_scripts_for_tree(ctx, ctx.root_id)
}

pub fn list_scripts_for_tree(ctx: &AppContext, id: i64) -> anyhow::Result<()> {
    let scripts = ctx.db.scripts_for_tree(id)?;
    if scripts.is_empty() {
        eprintln!("No scripts have been added yet. To add one, use okeep add.");
    } else {
        eprintln!("The following scripts are available (orun):\n");
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

pub fn list_files(ctx: &AppContext) -> anyhow::Result<()> {
    let files = ctx.db.files_for_tree(ctx.root_id)?;
    if files.is_empty() {
        eprintln!("No files have been saved yet. To add one, use okeep save.");
    } else {
        eprintln!("The following files are available (okeep restore):\n");
        for ScriptInfo { name, description } in files {
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

pub fn add_file(ctx: &mut AppContext, path: &str, bytes: Vec<u8>) -> anyhow::Result<()> {
    ctx.db.add_file(ctx.root_id, path, bytes)?;
    Ok(())
}

pub fn get_file(ctx: &mut AppContext, path: &str) -> anyhow::Result<Vec<u8>> {
    ctx.db.get_file_by_name(ctx.root_id, path)
}
