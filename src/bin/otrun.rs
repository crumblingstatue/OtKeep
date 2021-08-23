use std::ffi::OsStr;

use anyhow::{bail, Context};
use otkeep::{
    database::{NoSuchScriptForCurrentTree, ScriptInfo},
    AppContext,
};

fn main() {
    match try_main() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn try_main() -> anyhow::Result<i32> {
    let mut args = std::env::args_os().skip(1);
    let db = otkeep::load_db()?;
    let root_id = match otkeep::find_root(&db)? {
        Some((id, _)) => id,
        None => {
            bail!("No OtKeep tree root was found. To establish one, use otkeep establish");
        }
    };

    let mut app = AppContext { db, root_id };
    let cmd_name = match args.next() {
        Some(arg) => arg,
        None => {
            list_scripts(&mut app)?;
            eprintln!("\nFor more options, try otkeep",);
            return Ok(1);
        }
    };
    run(
        cmd_name.to_str().context("Command name not utf-8")?,
        &mut app,
        args,
    )
    .context("Failed to run script")
}

fn run(
    name: &str,
    ctx: &mut AppContext,
    args: impl Iterator<Item = impl AsRef<OsStr>>,
) -> anyhow::Result<i32> {
    match ctx.db.run_script(ctx.root_id, name, args) {
        Ok(status) => Ok(status.code().unwrap_or(1)),
        Err(e) => match e.downcast_ref::<NoSuchScriptForCurrentTree>() {
            Some(_) => {
                eprintln!("No script named '{}' for the current tree.\n", name);
                list_scripts(ctx)?;
                eprintln!("\nFor more options, try otkeep");
                Ok(1)
            }
            None => Err(e),
        },
    }
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