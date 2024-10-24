use std::{
    ffi::OsStr,
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    process::{Command, ExitStatus},
};

const MODE_EXEC: u32 = 0o755;

pub(crate) fn run_script(
    script: &[u8],
    args: impl Iterator<Item = impl AsRef<OsStr>>,
    tree_root: impl AsRef<OsStr>,
) -> anyhow::Result<ExitStatus> {
    let temp_dir = temp_dir::TempDir::new()?;
    let path = temp_dir.child("script");
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .mode(MODE_EXEC)
        .open(&path)?;
    f.write_all(script)?;
    let exit_status = Command::new(path)
        .env("OTKEEP_TREE_ROOT", tree_root)
        .args(args)
        .status()?;
    Ok(exit_status)
}
