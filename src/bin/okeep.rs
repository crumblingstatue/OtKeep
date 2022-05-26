#![feature(let_else)]

use std::path::PathBuf;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use otkeep::AppContext;

#[derive(Parser)]
#[clap(about, version)]
struct Args {
    #[clap(subcommand)]
    subcommand: Option<Sub>,
}

/// Out of tree keeper
#[derive(Subcommand)]
enum Sub {
    /// Adds a script for the current tree
    Add {
        /// The name the script will be referred to as
        name: String,
        /// A path to a script or an inline script
        script: String,
        /// Add an inline script instead of loading from a file
        #[clap(short = 'i', long = "inline")]
        inline: bool,
    },
    /// Modify the commands for the current tree
    Mod {
        /// Name of the script
        name: String,
        /// Add optional description for the command
        desc: Option<String>,
    },
    /// Remove a script
    Remove {
        /// Name of the script
        name: String,
    },
    /// Establish the current directory as a root
    Establish,
    /// Unestablish the current directory as a root
    Unestablish,
    /// List all the trees kept in the database
    ListTrees,
    /// Check out a copy of a script as a file
    Checkout {
        /// Name of the script
        name: String,
    },
    /// Concatenate a script to standard out
    Cat {
        /// Name of the script
        name: String,
    },
    /// Update a script with new contents
    Update {
        /// The of the script to update
        name: String,
        /// A path to a source script or an inline script
        script: String,
        /// Add an inline script instead of loading from a file
        #[clap(short = 'i', long = "inline")]
        inline: bool,
    },
    /// Rename a script
    Rename {
        /// The current name of the script
        current: String,
        /// The new name of the script
        new: String,
    },
    /// Save a file from the working tree
    Save {
        /// Path to the file
        path: String,
    },
    /// Restore a saved file to the working tree
    Restore {
        /// Path to the file
        path: Option<String>,
    },
    /// Clone all scripts from another tree
    Clone {
        /// Name of the tree to clone from
        tree: PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let db = otkeep::load_db()?;
    let opt_root = otkeep::find_root(&db)?;
    let Some(subcommand) = Args::parse().subcommand else {
        match opt_root {
            Some(root) => {
                let ctx = &AppContext {
                    db,
                    root_id: root.0,
                };
                otkeep::list_scripts(ctx)?;
                println!();
                otkeep::list_files(ctx)?;
                help_msg();
                return Ok(());
            }
            None => {
                eprintln!("The following trees are available:");
                cmd::list_trees(&db)?;
                help_msg();
                return Ok(());
            }
        }
    };

    match subcommand {
        Sub::ListTrees => {
            cmd::list_trees(&db)?;
            return Ok(());
        }
        Sub::Establish => {
            cmd::establish(&db).context("Failed to establish OtKeep root")?;
            eprintln!("Established {}", std::env::current_dir()?.display());
            return Ok(());
        }
        _ => {}
    }

    let (root_id, root_path) = match opt_root {
        Some(root) => root,
        None => {
            otkeep::print_established_trees(&db)?;
            bail!("No OtKeep tree root was found. To establish one, use okeep establish");
        }
    };

    let mut app = AppContext { db, root_id };
    match subcommand {
        Sub::Add {
            name,
            script,
            inline,
        } => cmd::add(&mut app, &name, &script, inline).context("Failed to add script")?,
        Sub::Mod { name, desc } => {
            cmd::mod_(&mut app, &name, desc.as_deref()).context("Mod failed")?
        }
        Sub::Remove { name } => cmd::remove(&mut app, &name).context("Failed to remove script")?,
        Sub::Establish => unreachable!(),
        Sub::Unestablish => {
            if std::env::current_dir()? != root_path {
                eprintln!("The current directory is not the root.");
                eprintln!("Go to {}", root_path.display());
                eprintln!("Then run this command again if you really want to unestablish");
                return Ok(());
            }
            cmd::unestablish(&mut app).context("Failed to unestablish current directory")?;
            eprintln!("Unestablished {}", root_path.display());
        }
        Sub::ListTrees => unreachable!(),
        Sub::Checkout { name } => cmd::checkout(&mut app, &name).context("Checkout failed")?,
        Sub::Cat { name } => cmd::cat(&mut app, &name).context("Cat failed")?,
        Sub::Update {
            name,
            script,
            inline,
        } => cmd::update(&mut app, &name, &script, inline).context("Update failed")?,
        Sub::Rename { current, new } => {
            cmd::rename(&mut app, &current, &new).context("Failed to rename script")?
        }
        Sub::Save { path } => cmd::save(&mut app, &path).context("File save failed")?,
        Sub::Restore { path } => {
            cmd::restore(&mut app, path.as_deref()).context("File restore failed")?
        }
        Sub::Clone { tree } => cmd::clone(&mut app, &tree)?,
    }
    Ok(())
}

fn help_msg() {
    eprintln!("\nType okeep --help for help.");
}

mod cmd {
    use std::path::Path;

    use anyhow::{bail, Context};

    use otkeep::{database::Database, AppContext};

    pub(crate) fn add(
        ctx: &mut AppContext,
        name: &str,
        script: &str,
        inline: bool,
    ) -> anyhow::Result<()> {
        let curr_dir = std::env::current_dir()?;
        let script_body = if inline {
            script.as_bytes().to_vec()
        } else {
            let absolute_path = std::fs::canonicalize(curr_dir.join(script))?;
            std::fs::read(&absolute_path)?
        };
        ctx.db.add_script(ctx.root_id, name, script_body)?;
        Ok(())
    }
    pub fn establish(db: &Database) -> anyhow::Result<()> {
        let current_dir = std::env::current_dir()?;
        match db.query_tree(&current_dir)? {
            None => db.add_new_tree(&current_dir)?,
            Some(_) => bail!("There is already a OtKeep tree root here."),
        }
        Ok(())
    }
    pub fn unestablish(ctx: &mut AppContext) -> anyhow::Result<()> {
        ctx.db.remove_tree(ctx.root_id)
    }
    pub fn mod_(ctx: &mut AppContext, name: &str, desc: Option<&str>) -> anyhow::Result<()> {
        let mut modded = false;

        if let Some(description) = desc {
            ctx.db
                .add_script_description(ctx.root_id, name, description)?;
            eprintln!("{} => {}", name, description);
            modded = true;
        }
        if !modded {
            eprintln!("No modification option given, did nothing.");
        }
        Ok(())
    }

    pub fn remove(ctx: &mut AppContext, name: &str) -> anyhow::Result<()> {
        if ctx.db.remove_script(ctx.root_id, name)? {
            eprintln!("Removed script '{}'", name);
        } else {
            eprintln!("Didn't remove anything. '{}' probably doesn't exist.", name);
        }
        Ok(())
    }

    pub fn list_trees(db: &Database) -> anyhow::Result<()> {
        let mut any = false;
        for root_path in db.get_tree_roots()? {
            eprintln!("{}", root_path.display());
            any = true;
        }
        if !any {
            eprintln!("Looks like no trees have been added yet.");
            eprintln!("Find a tree you'd like to add and type `okeep establish`.");
        }
        Ok(())
    }

    pub fn checkout(ctx: &mut AppContext, name: &str) -> anyhow::Result<()> {
        otkeep::checkout(name, ctx)?;
        Ok(())
    }

    pub fn cat(ctx: &mut AppContext, name: &str) -> anyhow::Result<()> {
        otkeep::cat(name, ctx)?;
        Ok(())
    }

    pub fn update(
        ctx: &mut AppContext,
        name: &str,
        script: &str,
        inline: bool,
    ) -> anyhow::Result<()> {
        let curr_dir = std::env::current_dir()?;
        let script_body = if inline {
            script.as_bytes().to_vec()
        } else {
            let absolute_path = std::fs::canonicalize(curr_dir.join(script))?;
            std::fs::read(&absolute_path)?
        };
        ctx.db.update_script(ctx.root_id, name, script_body)?;
        Ok(())
    }

    pub(crate) fn rename(ctx: &mut AppContext, current: &str, new: &str) -> anyhow::Result<()> {
        otkeep::rename_script(current, new, ctx)?;
        Ok(())
    }

    pub(crate) fn save(app: &mut AppContext, path: &str) -> anyhow::Result<()> {
        let bytes = std::fs::read(path)?;
        otkeep::add_file(app, path, bytes)?;
        Ok(())
    }

    pub(crate) fn restore(app: &mut AppContext, path: Option<&str>) -> anyhow::Result<()> {
        let path = match path {
            Some(path) => path,
            None => {
                otkeep::list_files(app)?;
                return Ok(());
            }
        };
        let bytes = otkeep::get_file(app, path)?;
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub(crate) fn clone(app: &mut AppContext, tree: &Path) -> anyhow::Result<()> {
        let dst = app.root_id;
        let src = app.db.query_tree(tree)?.context("Missing tree")?;
        app.db.clone_tree(src, dst)?;
        Ok(())
    }
}
