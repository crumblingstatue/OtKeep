use std::{
    ffi::OsStr,
    io::Write,
    os::{fd::FromRawFd, unix::process::CommandExt},
    process::Command,
};

pub(crate) fn run_script(
    script: &[u8],
    args: impl Iterator<Item = impl AsRef<OsStr>>,
    tree_root: impl AsRef<OsStr>,
) -> anyhow::Result<!> {
    extern "C" {
        fn memfd_create(name: *const std::ffi::c_char, flags: std::ffi::c_uint) -> std::ffi::c_int;
    }
    let fd = unsafe { memfd_create(c"otkeep-script".as_ptr(), 0) };
    if fd == -1 {
        anyhow::bail!("memfd_create failed when trying to create script file");
    }
    let mut f = unsafe { std::fs::File::from_raw_fd(fd) };
    f.write_all(script)?;
    f.flush()?;
    let err = Command::new(format!("/proc/self/fd/{fd}"))
        .env("OTKEEP_TREE_ROOT", tree_root)
        .args(args)
        .exec()
        .into();
    Err(err)
}
