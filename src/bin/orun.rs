#![feature(never_type)]

use {
    anyhow::{bail, Context},
    otkeep::{database::NoSuchScriptForCurrentTree, AppContext},
    std::ffi::OsStr,
};

fn main() {
    match try_main() {
        Err(e) => {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn try_main() -> anyhow::Result<!> {
    let mut args = std::env::args_os().skip(1);
    let db = otkeep::load_db()?;
    let root_id = match otkeep::find_root(&db)? {
        Some((id, _)) => id,
        None => {
            otkeep::print_established_trees(&db)?;
            bail!("No OtKeep tree root was found. To establish one, use okeep establish");
        }
    };

    let mut app = AppContext { db, root_id };
    let cmd_name = match args.next() {
        Some(arg) => arg,
        None => {
            otkeep::list_scripts(&app)?;
            eprintln!("\nFor more options, try okeep",);
            std::process::exit(1);
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
) -> anyhow::Result<!> {
    match ctx.db.run_script(ctx.root_id, name, args) {
        Err(e) => match e.downcast_ref::<NoSuchScriptForCurrentTree>() {
            Some(_) => {
                eprintln!("No script named '{}' for the current tree.\n", name);
                otkeep::list_scripts(ctx)?;
                eprintln!("\nFor more options, try okeep");
                std::process::exit(1)
            }
            None => Err(e),
        },
    }
}
