#[cfg(windows)]
pub use crate::w32::{
    Group, Passwd, getegid, geteuid, getgid, getgrgid, getpwnam, getpwuid, getuid,
};

#[cfg(unix)]
pub use emacs_unix::{
    Group, Passwd, getegid, geteuid, getgid, getgrgid, getpwnam, getpwuid, getuid,
};
