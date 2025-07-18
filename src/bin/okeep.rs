use {
    anyhow::{Context, bail},
    clap::{Parser, Subcommand},
    otkeep::AppContext,
    std::path::PathBuf,
};

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
        ///
        /// If not provided, $EDITOR will open to edit a new script
        script: Option<String>,
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
    #[clap(alias = "rm")]
    Remove {
        /// Name of the script
        name: String,
    },
    /// Establish the current directory as a root
    Establish,
    /// Unestablish the current directory as a root
    Unestablish,
    /// Reestablish (move) another root to the current directory
    Reestablish { old_root: PathBuf },
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
    /// Clone a single script from a path
    Cp {
        /// Path to the tree
        tree: PathBuf,
        /// Name of the script
        name: String,
    },
    /// Clone all scripts from another tree
    Clone {
        /// Path to the tree
        tree: PathBuf,
    },
    /// List scripts from a tree
    ListScripts {
        /// Path to the tree
        tree: PathBuf,
    },
    /// Edit a script. Uses editor from $EDITOR env var.
    Edit {
        /// Name of the script
        name: String,
    },
    /// Interactively remove unused things
    #[clap(subcommand)]
    Prune(PruneSubCmd),
}

#[derive(Subcommand)]
enum PruneSubCmd {
    /// Interactively remove old trees that don't exist on the filesystem
    Trees,
    /// Interactively remove old blobs that aren't referenced by any trees
    Blobs,
}

fn main() -> anyhow::Result<()> {
    let mut db = otkeep::load_db()?;
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
        Sub::Reestablish { ref old_root } => {
            cmd::reestablish(&db, old_root).context("Failed to reestablish OtKeep root")?;
            eprintln!(
                "Reestablished {} as {}",
                old_root.display(),
                std::env::current_dir()?.display()
            );
            return Ok(());
        }
        Sub::Prune(PruneSubCmd::Trees) => {
            let mut any_was_stray = false;
            for root in db.get_tree_roots()? {
                if !root.path.exists() {
                    any_was_stray = true;
                    eprintln!("`{}` has the following scripts: ", root.path.display());
                    for script in db.scripts_for_tree(root.id)? {
                        eprintln!("{}", script.name);
                    }
                    let files = db.files_for_tree(root.id)?;
                    if !files.is_empty() {
                        eprintln!("... and following files: ");
                        for file in files {
                            eprintln!("{}", file.name);
                        }
                    }
                    eprintln!("Remove? (y/n)");
                    let mut ans_line = String::new();
                    std::io::stdin().read_line(&mut ans_line)?;
                    let ans = ans_line.trim();
                    if ans == "y" {
                        db.remove_tree(root.id)?;
                    }
                }
            }
            if !any_was_stray {
                eprintln!("No stray roots were detected.");
            }
            return Ok(());
        }
        Sub::Prune(PruneSubCmd::Blobs) => {
            let mut any_was_stray_and_nonnull = false;
            let tree_blob_refs = db.tree_script_blob_ids()?;
            let len = db.blobs_table_len()?;
            for rowid in 1..=len {
                if !tree_blob_refs.contains(&rowid) {
                    if db.blob_is_null(rowid)? {
                        continue;
                    }
                    any_was_stray_and_nonnull = true;
                    let data = db.fetch_blob(rowid)?;
                    let s = String::from_utf8_lossy(&data);
                    eprintln!("Unreferenced blob:");
                    eprintln!("{s}");
                    eprintln!("Remove? (y/n)");
                    let mut ans_line = String::new();
                    std::io::stdin().read_line(&mut ans_line)?;
                    let ans = ans_line.trim();
                    if ans == "y" {
                        db.nullify_blob(rowid)?;
                    }
                }
            }
            if !any_was_stray_and_nonnull {
                eprintln!("No stray blobs were detected.");
            }
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
        // We matched against these eariler
        Sub::Establish | Sub::Reestablish { .. } | Sub::ListTrees | Sub::Prune(_) => unreachable!(),
        Sub::Add {
            name,
            script,
            inline,
        } => {
            cmd::add(&mut app, &name, script.as_deref(), inline).context("Failed to add script")?
        }
        Sub::Mod { name, desc } => {
            cmd::mod_(&mut app, &name, desc.as_deref()).context("Mod failed")?
        }
        Sub::Remove { name } => cmd::remove(&mut app, &name).context("Failed to remove script")?,
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
        Sub::ListScripts { tree } => {
            match otkeep::find_root_for_path(&app.db, &tree)? {
                Some((root_id, _)) => otkeep::list_scripts_for_tree(&app, root_id)?,
                None => {
                    eprintln!("No root found at the given location ({})", tree.display());
                }
            };
        }
        Sub::Cp { tree, name } => match otkeep::find_root_for_path(&app.db, &tree)? {
            Some((other_tree_id, _)) => {
                let blob = app.db.get_script_by_name(other_tree_id, &name)?;
                app.db.add_script(root_id, &name, blob)?;
            }
            None => {
                eprintln!("No root found at the given location ({})", tree.display());
            }
        },
        Sub::Edit { name } => {
            let Some(editor) = std::env::var_os("EDITOR") else {
                eprintln!("$EDITOR env var needs to be set to edit");
                return Ok(());
            };
            let blob = app.db.get_script_by_name(root_id, &name)?;
            let dir = temp_dir::TempDir::new()?;
            let filepath = dir.path().join("okeep-script.txt");
            std::fs::write(&filepath, blob)?;
            std::process::Command::new(editor).arg(&filepath).status()?;
            let blob = std::fs::read(&filepath)?;
            app.db.update_script(root_id, &name, blob)?;
        }
    }
    Ok(())
}

fn help_msg() {
    eprintln!("\nType okeep --help for help.");
}

mod cmd {
    use {
        anyhow::{Context, bail},
        otkeep::{AppContext, database::Database},
        owo_colors::{OwoColorize, Style},
        std::path::Path,
    };

    pub(crate) fn add(
        ctx: &mut AppContext,
        name: &str,
        script: Option<&str>,
        mut inline: bool,
    ) -> anyhow::Result<()> {
        let script_buf;
        let script = match script {
            Some(s) => s,
            None => {
                inline = true;
                let Some(editor) = std::env::var_os("EDITOR") else {
                    bail!("No $EDITOR set. Can't edit script");
                };
                let dir = temp_dir::TempDir::new()?;
                let filepath = dir.child("script.txt");
                std::process::Command::new(editor)
                    .arg(&filepath)
                    .status()
                    .context("Launching editor")?;
                script_buf = std::fs::read_to_string(filepath).context("Reading script file")?;
                &script_buf
            }
        };
        let curr_dir = std::env::current_dir()?;
        let script_body = if inline {
            script.as_bytes().to_vec()
        } else {
            let absolute_path = std::fs::canonicalize(curr_dir.join(script))?;
            std::fs::read(absolute_path)?
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
    pub fn reestablish(db: &Database, old_root: &Path) -> anyhow::Result<()> {
        let current_dir = std::env::current_dir()?;
        match db.query_tree(&current_dir)? {
            None => {
                db.rename_tree(old_root, &current_dir)?;
            }
            Some(_) => bail!("There is already a OtKeep tree root here."),
        }
        Ok(())
    }
    pub fn mod_(ctx: &mut AppContext, name: &str, desc: Option<&str>) -> anyhow::Result<()> {
        let mut modded = false;

        if let Some(description) = desc {
            ctx.db
                .add_script_description(ctx.root_id, name, description)?;
            eprintln!("{name} => {description}");
            modded = true;
        }
        if !modded {
            eprintln!("No modification option given, did nothing.");
        }
        Ok(())
    }

    pub fn remove(ctx: &mut AppContext, name: &str) -> anyhow::Result<()> {
        if ctx.db.remove_script(ctx.root_id, name)? {
            eprintln!("Removed script '{name}'");
        } else {
            eprintln!("Didn't remove anything. '{name}' probably doesn't exist.");
        }
        Ok(())
    }

    pub fn list_trees(db: &Database) -> anyhow::Result<()> {
        let mut any = false;
        for root in db.get_tree_roots()? {
            let mut style = Style::new();
            if !root.path.exists() {
                style = style.bright_black();
            }
            eprintln!("{}", root.path.display().style(style));
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
            std::fs::read(absolute_path)?
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
