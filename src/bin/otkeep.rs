use anyhow::{bail, Context};
use clap::{App, Arg, SubCommand};
use otkeep::AppContext;

fn main() -> anyhow::Result<()> {
    let matches = App::new("otkeep")
        .about("Out of tree keeper")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("add")
                .about("Adds a script for the current tree")
                .arg(
                    Arg::with_name("name")
                        .help("The name the script will be referred to as")
                        .required(true),
                )
                .arg(
                    Arg::with_name("script")
                        .required(true)
                        .help("A path to a script or an inline script"),
                )
                .arg(
                    Arg::with_name("inline")
                        .short("i")
                        .long("--inline")
                        .takes_value(false)
                        .help("Add an inline script instead of loading from a file"),
                ),
        )
        .subcommand(
            SubCommand::with_name("mod")
                .about("Modify the commands for the current tree")
                .arg(
                    Arg::with_name("name")
                        .help("Name of the script")
                        .required(true),
                )
                .arg(
                    Arg::with_name("description")
                        .long("description")
                        .takes_value(true)
                        .help("Add optional description for the command"),
                ),
        )
        .subcommand(
            SubCommand::with_name("remove")
                .about("Remove a script")
                .arg(
                    Arg::with_name("name")
                        .help("Name of the script")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("establish").about("Establish the current directory as a root"),
        )
        .subcommand(
            SubCommand::with_name("unestablish")
                .about("Unestablish the current directory as a root"),
        )
        .subcommand(
            SubCommand::with_name("list-trees").about("List all the trees kept in the database"),
        )
        .get_matches();
    let (name, matches) = matches.subcommand();
    let matches = matches.context("No subcommand matches")?;

    let db = otkeep::load_db()?;
    let opt_root = otkeep::find_root(&db)?;
    if name == "establish" {
        cmd::establish(&db).context("Failed to establish OtKeep root")?;
        eprintln!("Established {}", std::env::current_dir()?.display());
        return Ok(());
    }
    let (root_id, root_path) = match opt_root {
        Some(root) => root,
        None => {
            bail!("No OtKeep tree root was found. To establish one, use otkeep establish");
        }
    };

    let mut app = AppContext { db, root_id };
    match name {
        "add" => cmd::add(matches, &mut app).context("Failed to add script")?,
        "mod" => cmd::mod_(matches, &mut app).context("Mod command failed")?,
        "remove" => cmd::remove(matches, &mut app).context("Failed to remove script")?,
        "unestablish" => {
            if std::env::current_dir()? != root_path {
                eprintln!("The current directory is not the root.");
                eprintln!("Go to {}", root_path.display());
                eprintln!("Then run this command again if you really want to unestablish");
                return Ok(());
            }
            cmd::unestablish(&mut app).context("Failed to unestablish current directory")?;
            eprintln!("Unestablished {}", root_path.display());
        }
        "list-trees" => cmd::list_trees(&mut app)?,
        _ => {
            bail!("Invalid subcommand: '{}'", name);
        }
    }
    Ok(())
}

mod cmd {
    use anyhow::{bail, Context};
    use clap::ArgMatches;

    use otkeep::{database::Database, AppContext};

    pub(crate) fn add(matches: &ArgMatches, ctx: &mut AppContext) -> anyhow::Result<()> {
        let script_arg = matches.value_of("script").context("Missing script file")?;
        let name = matches.value_of("name").context("Missing name")?;
        let inline = matches.is_present("inline");
        let curr_dir = std::env::current_dir()?;
        let script_body = if inline {
            script_arg.as_bytes().to_vec()
        } else {
            let absolute_path = std::fs::canonicalize(curr_dir.join(script_arg))?;
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
    pub fn mod_(matches: &ArgMatches, ctx: &mut AppContext) -> anyhow::Result<()> {
        let name_arg = matches.value_of("name").context("Missing script name")?;
        let mut modded = false;

        if let Some(description) = matches.value_of("description") {
            ctx.db
                .add_script_description(ctx.root_id, name_arg, description)?;
            eprintln!("{} => {}", name_arg, description);
            modded = true;
        }
        if !modded {
            eprintln!("No modification option given, did nothing.");
        }
        Ok(())
    }

    pub fn remove(matches: &ArgMatches, ctx: &mut AppContext) -> anyhow::Result<()> {
        let name_arg = matches.value_of("name").context("Missing script name")?;
        if ctx.db.remove_script(ctx.root_id, name_arg)? {
            eprintln!("Removed script '{}'", name_arg);
        } else {
            eprintln!(
                "Didn't remove anything. '{}' probably doesn't exist.",
                name_arg
            );
        }
        Ok(())
    }

    pub fn list_trees(ctx: &mut AppContext) -> anyhow::Result<()> {
        let mut any = false;
        for root_path in ctx.db.get_tree_roots()? {
            eprintln!("{}", root_path.display());
            any = true;
        }
        if !any {
            eprintln!("Looks like no trees have been added yet.");
            eprintln!("Find a tree you'd like to add and type `otkeep establish`.");
        }
        Ok(())
    }
}