//! This module defines the windows thread control.
//!
//! The crate's prelude doesn't have much control over
//! the windows threads, and this module provides
//! better control over those.

use winapi::ctypes::c_int;
use winapi::um::processthreadsapi::{GetCurrentThread, SetThreadPriority};
use winapi::um::winnt::HANDLE;

use crate::{Error, ThreadPriority};

/// The WinAPI priority representation. Check out MSDN for more info:
/// https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-setthreadpriority
pub enum WinAPIThreadPriority {
    /// Begin background processing mode. The system lowers the resource
    /// scheduling priorities of the thread so that it can perform background
    /// work without significantly affecting activity in the foreground.
    ///
    /// This value can be specified only if hThread is a handle to the current
    /// thread. The function fails if the thread is already in background processing mode.
    ///
    /// # Warning
    /// Windows Server 2003: This value is not supported.
    BackgroundModeBegin = 0x0001_0000,
    /// End background processing mode. The system restores the resource
    /// scheduling priorities of the thread as they were before the thread
    /// entered background processing mode.
    ///
    /// This value can be specified only if hThread is a handle to the current thread.
    /// The function fails if the thread is not in background processing mode.
    ///
    /// # Warning
    /// Windows Server 2003: This value is not supported.
    BackgroundModeEnd = 0x0002_0000,
    /// Priority 1 point above the priority class.
    AboveNormal = 1,
    /// Priority 1 point below the priority class.
    BelowNormal = -1,
    /// Priority 2 points above the priority class.
    Highest = 2,
    /// Base priority of 1 for IDLE_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS,
    /// NORMAL_PRIORITY_CLASS, ABOVE_NORMAL_PRIORITY_CLASS, or HIGH_PRIORITY_CLASS
    /// processes, and a base priority of 16 for REALTIME_PRIORITY_CLASS processes.
    Idle = -15,
    /// Priority 2 points below the priority class.
    Lowest = -2,
    /// Normal priority for the priority class.
    Normal = 0,
    /// Base priority of 15 for IDLE_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS,
    /// NORMAL_PRIORITY_CLASS, ABOVE_NORMAL_PRIORITY_CLASS, or HIGH_PRIORITY_CLASS
    /// processes, and a base priority of 31 for REALTIME_PRIORITY_CLASS processes.
    TimeCritical = 15,
}

impl std::convert::TryFrom<ThreadPriority> for WinAPIThreadPriority {
    type Error = crate::Error;

    fn try_from(priority: ThreadPriority) -> Result<Self, Self::Error> {
        Ok(match priority {
            ThreadPriority::Min => WinAPIThreadPriority::Lowest,
            ThreadPriority::Specific(p) => match p {
                0 => WinAPIThreadPriority::Idle,
                1..=19 => WinAPIThreadPriority::Lowest,
                21..=39 => WinAPIThreadPriority::BelowNormal,
                41..=59 => WinAPIThreadPriority::Normal,
                61..=79 => WinAPIThreadPriority::AboveNormal,
                81..=99 => WinAPIThreadPriority::Highest,
                _ => return Err(Error::Priority("The value is out of range [0; 99]")),
            },
            ThreadPriority::Max => WinAPIThreadPriority::Highest,
        })
    }
}

/// Sets thread's priority and schedule policy
///
/// * May require privileges
///
/// # Usage
///
/// Setting thread priority to minimum with normal schedule policy:
///
/// ```rust
/// use thread_priority::*;
///
/// let thread_id = thread_native_id();
/// assert!(set_thread_priority(thread_id, ThreadPriority::Min).is_ok());
/// ```
///
/// If there's an error, the result of
/// [`GetLastError`](https://docs.microsoft.com/en-us/windows/win32/api/errhandlingapi/nf-errhandlingapi-getlasterror) is returned.
pub fn set_thread_priority(native: HANDLE, priority: ThreadPriority) -> Result<(), Error> {
    use std::convert::TryFrom;
    use winapi::um::errhandlingapi::GetLastError;

    unsafe {
        if SetThreadPriority(native, WinAPIThreadPriority::try_from(priority)? as c_int) != 0 {
            Ok(())
        } else {
            Err(Error::OS(GetLastError() as i32))
        }
    }
}

/// Set current thread's priority.
pub fn set_current_thread_priority(priority: ThreadPriority) -> Result<(), Error> {
    let thread_id = thread_native_id();
    set_thread_priority(thread_id, priority)
}

/// Returns current thread id, which is the current OS's native handle.
/// It may or may not be equal or even related to rust's thread id,
/// there is absolutely no guarantee for that.
///
/// # Usage
///
/// ```rust
/// use thread_priority::thread_native_id;
///
/// assert!(!thread_native_id().is_null());
/// ```
pub fn thread_native_id() -> HANDLE {
    unsafe { GetCurrentThread() }
}
