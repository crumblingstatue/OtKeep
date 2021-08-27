# OtKeep - Out of tree keeper

OtKeep is a tool that helps you manage out of tree scripts for different projects.

## What problem does it solve?
OtKeep stores all your out of tree scripts in its database, letting you use them conveniently,
just as easily as if you had them in-tree.

What are out of tree scripts? They are personal scripts you make for a project that you don't
want to commit. Here are some examples:

- Running a project in very specific configurations

  Setting different arguments, environment variables, etc.
  You want to put this into a script so you don't have to
  remember and type all of this stuff up every time.

- Building a project in very specific configurations

  e.g. invoking cmake with the `i686-pc-windows-gnu` cross-compiler, while also cross-compiling
  all the dependencies, and pointing to all the cross-compiled libraries on your system.

- Packaging up a project for your friends to try out

  This might involve cross-compiling to their OS, copying over .dll files and assets, then zipping it all up

- Etc.

  I'm sure you can figure out more use cases. I use them a lot
  for personal convenience. I'm not here to sell the idea of
  out of tree scripts,  I'm here to sell the idea of a tool that
  manages them.

You don't want to commit these kinds of scripts into a repository, because they are highly specific to you.

What about just adding them to .gitignore or such?
The problem with that is that they can be easily destroyed by a git clean -dfx, or reclone of the repository.
And they still dirty up the working tree. They are not organized nicely.

Wouldn't it be nicer to store them in some kind of database in your user directory?
This is exactly what OtKeep is for.

Needless to say, if you don't have to deal with out of tree scripts, then OtKeep is not for you.

## Usage

OTKeep provides 2 tools, `otkeep` for managing your scripts, and `otrun` for running them.

### Adding scripts
To add a script, use `otkeep add`.
For example, to add your windows cross-build script called `build_win.sh`, do `otkeep add build-win build_win.sh`.

### Running scripts
To run script you added, be in the tree you added it to, and simply run `otrun` with the script name as argument.
For the aformentioned `build-win` example, you would run `otrun build-win`.
`otrun` forwards all arguments to the script.

### Listing scripts for the current tree
Simply run `otrun` without any arguments. It will list the scripts available for the current tree.
