use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

pub fn from_raw(bytes: &[u8]) -> anyhow::Result<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    Ok(Path::new(OsStr::from_bytes(bytes)).to_owned())
}

pub fn to_raw(path: &Path) -> &[u8] {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes()
}
