use {
    crate::script_ext,
    std::{
        ffi::OsStr,
        process::{Command, ExitStatus},
    },
};

pub(crate) fn run_script(
    script: &[u8],
    args: impl Iterator<Item = impl AsRef<OsStr>>,
) -> anyhow::Result<ExitStatus> {
    let temp_dir = temp_dir::TempDir::new()?;
    let path = temp_dir.child(format!("script.{}", script_ext()));
    std::fs::write(&path, script)?;
    Ok(script_command(|cmd| cmd.arg(path).args(args).status())?)
}

type CmdResult = std::io::Result<ExitStatus>;

#[cfg(not(target_os = "windows"))]
fn script_command<F: FnOnce(&mut Command) -> CmdResult>(f: F) -> CmdResult {
    let mut cmd = Command::new("sh");
    f(&mut cmd)
}

#[cfg(target_os = "windows")]
fn script_command<F: FnOnce(&mut Command) -> CmdResult>(f: F) -> CmdResult {
    let mut cmd = Command::new("cmd");
    cmd.arg("/c");
    f(&mut cmd)
}
