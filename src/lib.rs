//! Thread priority. A library for changing thread's priority.
//!
//! # Usage
//!
//! Setting thread priority to minimum:
//!
//! ```rust
//! use thread_priority::*;
//!
//! assert!(set_current_thread_priority(ThreadPriority::Min).is_ok());
//! // Or like this:
//! assert!(ThreadPriority::Min.set_for_current().is_ok());
//! ```
#![warn(missing_docs)]
#![deny(warnings)]

#[cfg(unix)]
pub mod unix;
#[cfg(unix)]
pub use unix::*;
#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;

/// A error type
#[derive(Debug, Copy, Clone)]
pub enum Error {
    /// A value which describes why it is impossible to use such a priority.
    Priority(&'static str),
    /// Target OS' error type. In most systems it is an integer which
    /// later should be used with target OS' API for understanding the value.
    OS(i32),
    /// FFI failure
    Ffi(&'static str),
}

/// Thread priority enumeration.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThreadPriority {
    /// Holds a value representing the minimum possible priority.
    Min,
    /// Holds a specific priority value. Should be in [0; 100] range,
    /// a percentage value.
    Specific(u8),
    /// Holds a value representing the maximum possible priority.
    /// Should be used with caution, it solely depends on the target
    /// os where the program is going to be running on, how it will
    /// behave. On some systems, the whole system may become frozen
    /// if not used properly.
    Max,
}

impl ThreadPriority {
    /// Sets current thread's priority to this value.
    pub fn set_for_current(self) -> Result<(), Error> {
        set_current_thread_priority(self)
    }
}
