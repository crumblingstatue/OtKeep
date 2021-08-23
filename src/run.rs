use std::{
    ffi::OsStr,
    process::{Command, ExitStatus},
};

pub(crate) fn run_script(
    script: &[u8],
    args: impl Iterator<Item = impl AsRef<OsStr>>,
) -> anyhow::Result<ExitStatus> {
    let temp_dir = temp_dir::TempDir::new()?;
    let path = temp_dir.child("script");
    std::fs::write(&path, script)?;
    Ok(Command::new("sh").arg(path).args(args).status()?)
}
