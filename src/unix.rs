//! This module defines the unix thread control.
//!
//! The crate's prelude doesn't have much control over
//! the unix threads, and this module provides
//! better control over those.

use crate::{Error, ThreadPriority};
pub use libc::sched_param as ScheduleParams;

/// The following "real-time" policies are also supported, for special time-critical applications
/// that need precise control over the way in which runnable processes are selected for execution
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum RealtimeThreadSchedulePolicy {
    /// A first-in, first-out policy
    Fifo,
    /// A round-robin policy
    RoundRobin,
}
impl RealtimeThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            RealtimeThreadSchedulePolicy::Fifo => 1,
            RealtimeThreadSchedulePolicy::RoundRobin => 2,
        }
    }
}

/// Normal (usual) schedule policies
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum NormalThreadSchedulePolicy {
    /// For running very low priority background jobs
    Idle,
    /// For "batch" style execution of processes
    Batch,
    /// The standard round-robin time-sharing policy
    Other,
    /// The standard round-robin time-sharing policy
    Normal,
}
impl NormalThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            NormalThreadSchedulePolicy::Idle => 5,
            NormalThreadSchedulePolicy::Batch => 3,
            NormalThreadSchedulePolicy::Other | NormalThreadSchedulePolicy::Normal => 0,
        }
    }
}

/// Thread schedule policy definition
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThreadSchedulePolicy {
    /// Normal thread schedule policies
    Normal(NormalThreadSchedulePolicy),
    /// Realtime thread schedule policies
    Realtime(RealtimeThreadSchedulePolicy),
}
impl ThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            ThreadSchedulePolicy::Normal(p) => p.to_posix(),
            ThreadSchedulePolicy::Realtime(p) => p.to_posix(),
        }
    }

    fn from_posix(policy: libc::c_int) -> Result<ThreadSchedulePolicy, Error> {
        match policy {
            0 => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Normal,
            )),
            3 => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Batch,
            )),
            5 => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Idle,
            )),
            1 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Fifo,
            )),
            2 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::RoundRobin,
            )),
            _ => Err(Error::Ffi("Can't parse schedule policy from posix")),
        }
    }
}

impl ThreadPriority {
    /// POSIX value can not be known without knowing the scheduling policy
    /// https://linux.die.net/man/2/sched_get_priority_max
    pub fn to_posix(self, policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        let ret = match self {
            ThreadPriority::Min => match policy {
                ThreadSchedulePolicy::Realtime(_) => Ok(1),
                _ => Ok(0),
            },
            ThreadPriority::Specific(p) => match policy {
                ThreadSchedulePolicy::Realtime(_) if (p == 0 || p > 99) => {
                    Err(Error::Priority("The value is out of range [0; 99]"))
                }
                ThreadSchedulePolicy::Normal(_) if p != 0 => Err(Error::Priority(
                    "The value can be only 0 for normal scheduling policy",
                )),
                _ => Ok(p),
            },
            ThreadPriority::Max => match policy {
                ThreadSchedulePolicy::Realtime(_) => Ok(99),
                _ => Ok(0),
            },
        };
        ret.map(|p| p as libc::c_int)
    }

    /// Sets current thread's priority to this value.
    pub fn set_for_current(self) -> Result<(), Error> {
        set_current_thread_priority(self)
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
/// assert!(set_thread_priority_and_policy(thread_id,
///                                        ThreadPriority::Min,
///                                        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
/// ```
pub fn set_thread_priority_and_policy(
    native: libc::pthread_t,
    priority: ThreadPriority,
    policy: ThreadSchedulePolicy,
) -> Result<(), Error> {
    let params = ScheduleParams {
        sched_priority: priority.to_posix(policy)?,
    };
    set_thread_schedule_policy(native, policy, params)
}

/// Set current thread's priority.
pub fn set_current_thread_priority(priority: ThreadPriority) -> Result<(), Error> {
    let thread_id = thread_native_id();
    let policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal);
    set_thread_priority_and_policy(thread_id, priority, policy)
}

/// Returns policy parameters (schedule policy and other schedule parameters) for current process
///
/// # Usage
///
/// ```rust
/// use thread_priority::*;
///
/// assert!(thread_schedule_policy().is_ok());
/// ```
pub fn thread_schedule_policy() -> Result<ThreadSchedulePolicy, Error> {
    unsafe { ThreadSchedulePolicy::from_posix(libc::sched_getscheduler(libc::getpid())) }
}

/// Sets thread schedule policy.
///
/// * May require privileges
///
/// # Usage
/// ```rust,no_run
/// use thread_priority::*;
///
/// let thread_id = thread_native_id();
/// let policy = ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo);
/// let params = ScheduleParams { sched_priority: 3 as libc::c_int };
/// assert!(set_thread_schedule_policy(thread_id, policy, params).is_ok());
/// ```
pub fn set_thread_schedule_policy(
    native: libc::pthread_t,
    policy: ThreadSchedulePolicy,
    params: ScheduleParams,
) -> Result<(), Error> {
    unsafe {
        let ret = libc::pthread_setschedparam(
            native,
            policy.to_posix(),
            &params as *const ScheduleParams,
        );
        match ret {
            0 => Ok(()),
            e => Err(Error::OS(e)),
        }
    }
}

/// Returns policy parameters (schedule policy and other schedule parameters)
///
/// # Usage
///
/// ```rust
/// use thread_priority::*;
///
/// let thread_id = thread_native_id();
/// assert!(thread_schedule_policy_param(thread_id).is_ok());
/// ```
pub fn thread_schedule_policy_param(
    native: libc::pthread_t,
) -> Result<(ThreadSchedulePolicy, ScheduleParams), Error> {
    unsafe {
        let mut policy = 0 as libc::c_int;
        let mut params = ScheduleParams { sched_priority: 0 };

        let ret = libc::pthread_getschedparam(
            native,
            &mut policy as *mut libc::c_int,
            &mut params as *mut ScheduleParams,
        );
        match ret {
            0 => Ok((ThreadSchedulePolicy::from_posix(policy)?, params)),
            e => Err(Error::OS(e)),
        }
    }
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
/// assert!(thread_native_id() > 0);
/// ```
pub fn thread_native_id() -> libc::pthread_t {
    unsafe { libc::pthread_self() }
}

#[cfg(test)]
mod tests {
    use crate::unix::*;

    #[test]
    fn thread_schedule_policy_param_test() {
        let thread_id = thread_native_id();

        assert!(thread_schedule_policy_param(thread_id).is_ok());
    }

    #[test]
    fn set_thread_priority_test() {
        let thread_id = thread_native_id();

        assert!(set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Min,
            ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)
        )
        .is_ok());
        assert!(set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Max,
            ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)
        )
        .is_ok());
        assert!(set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Specific(0),
            ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)
        )
        .is_ok());
    }
}