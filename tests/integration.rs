#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd"
))]
mod unix;
#[cfg(windows)]
mod windows;
