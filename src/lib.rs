//! Thread priority. A library for changing thread's priority.
//!
//! Uses `libpthread` to work with threads.
//!
//! # Usage
//! 
//! Setting thread priority to minimum:
//! 
//! ```rust
//! extern crate thread_priority;
//! use thread_priority::*;
//! 
//! fn main() {
//!     let thread_id = thread_native_id();
//!     assert!(set_thread_priority(thread_id,
//!                                 ThreadPriority::Min,
//!                                 ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
//! }
//! ```

extern crate libc;


/// A error type
#[derive(Debug, Copy, Clone)]
pub enum Error {
    /// A value which describes why it is impossible to use such a priority
    Priority(&'static str),
    /// Pthread error type
    Pthread(i32),
    /// FFI failure
    Ffi(&'static str),
}

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
    fn to_posix(&self) -> libc::c_int {
        match *self {
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
    fn to_posix(&self) -> libc::c_int {
        match *self {
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
    fn to_posix(&self) -> libc::c_int {
        match *self {
            ThreadSchedulePolicy::Normal(p) => p.to_posix(),
            ThreadSchedulePolicy::Realtime(p) => p.to_posix(),
        }
    }

    fn from_posix(policy: libc::c_int) -> Result<ThreadSchedulePolicy, Error> {
        match policy {
            0 => Ok(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)),
            3 => Ok(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch)),
            5 => Ok(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle)),
            1 => Ok(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo)),
            2 => Ok(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin)),
            _ => Err(Error::Ffi("Can't parse schedule policy from posix")),
        }
    }
}

/// Thread priority enumeration
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThreadPriority {
    Min,
    Specific(u8),
    Max,
}
impl ThreadPriority {
    /// POSIX value can not be known without knowing the scheduling policy
    /// https://linux.die.net/man/2/sched_get_priority_max
    fn to_posix(&self, policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        let ret = match *self {
            ThreadPriority::Min => {
                match policy {
                    ThreadSchedulePolicy::Realtime(_) => Ok(1),
                    _ => Ok(0)
                }
            },
            ThreadPriority::Specific(p) => {
                match policy {
                    ThreadSchedulePolicy::Realtime(_) if (p == 0 || p > 99) => {
                        Err(Error::Priority("The value is out of range [0; 99]"))
                    },
                    ThreadSchedulePolicy::Normal(_) if p != 0 => {
                        Err(Error::Priority("The value can be only 0 for normal scheduling policy"))
                    },
                    _ => Ok(p)
                }
            },
            ThreadPriority::Max => {
                match policy {
                    ThreadSchedulePolicy::Realtime(_) => Ok(99),
                    _ => Ok(0),
                }
            },
        };
        ret.map(|p| p as libc::c_int)
    }
}

/// Sets thread's priority and schedule policy
/// 
/// # Usage
/// 
/// Setting thread priority to minimum with normal schedule policy:
/// 
/// ```rust
/// extern crate thread_priority;
/// use thread_priority::*;
/// 
/// fn main() {
///     let thread_id = thread_native_id();
///     assert!(set_thread_priority(thread_id,
///                                 ThreadPriority::Min,
///                                 ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
/// }
/// ```
pub fn set_thread_priority(native: libc::pthread_t,
                           priority: ThreadPriority,
                           policy: ThreadSchedulePolicy) -> Result<(), Error> {
    unsafe {
        match libc::pthread_setschedprio(native, priority.to_posix(policy)?) {
            0 => Ok(()),
            e => Err(Error::Pthread(e)),
        }
    }
}

/// Returns current thread id (pthread)
/// 
/// # Usage
/// 
/// ```rust
/// extern crate thread_priority;
/// use thread_priority::*;
/// 
/// fn main() {
///     assert!(thread_native_id() > 0);
/// }
/// ```
pub fn thread_native_id() -> libc::pthread_t {
    unsafe {
        libc::pthread_self()
    }
}

/// Returns policy parameters (schedule policy and other schedule parameters) for current process
/// 
/// # Usage
/// 
/// ```rust
/// extern crate thread_priority;
/// use thread_priority::*;
/// 
/// fn main() {
///     assert!(thread_schedule_policy().is_ok());
/// }
/// ```
pub fn thread_schedule_policy() -> Result<ThreadSchedulePolicy, Error> {
    unsafe {
        ThreadSchedulePolicy::from_posix(libc::sched_getscheduler(libc::getpid()))
    }
}

/// Sets thread schedule policy.
/// 
/// * May require privileges
/// 
/// # Usage
/// ```rust,no_run
/// extern crate thread_priority;
/// extern crate libc;
/// 
/// use thread_priority::*;
/// 
/// fn main() {
///     let thread_id = thread_native_id();
///     let policy = ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo);
///     let params = ScheduleParams { sched_priority: 3 as libc::c_int };
///     assert!(set_thread_schedule_policy(thread_id, policy, params).is_ok());
/// }
/// ```
pub fn set_thread_schedule_policy(native: libc::pthread_t,
                                  policy: ThreadSchedulePolicy,
                                  params: ScheduleParams) -> Result<(), Error> {
    unsafe {
        let ret = libc::pthread_setschedparam(native,
                                              policy.to_posix(),
                                              &params as *const ScheduleParams);
        match ret {
            0 => Ok(()),
            e => Err(Error::Pthread(e)),
        }
    }
}

/// Returns policy parameters (schedule policy and other schedule parameters)
/// 
/// # Usage
/// 
/// ```rust
/// extern crate thread_priority;
/// use thread_priority::*;
/// 
/// fn main() {
///     let thread_id = thread_native_id();
///     assert!(thread_schedule_policy_param(thread_id).is_ok());
/// }
/// ```
pub fn thread_schedule_policy_param(native: libc::pthread_t) -> Result<(ThreadSchedulePolicy,
                                                                        ScheduleParams), Error> {
    unsafe {
        let mut policy = 0 as libc::c_int;
        let mut params = ScheduleParams { sched_priority: 0 };

        let ret = libc::pthread_getschedparam(native,
                                              &mut policy as *mut libc::c_int,
                                              &mut params as *mut ScheduleParams);
        match ret {
            0 => Ok((ThreadSchedulePolicy::from_posix(policy)?, params)),
            e => Err(Error::Pthread(e)),
        }
    }
}


#[cfg(test)]
mod tests {
    use ::*;

    #[test]
    fn thread_schedule_policy_param_test() {
        let thread_id = thread_native_id();

        assert!(thread_schedule_policy_param(thread_id).is_ok());
    }

    #[test]
    fn set_thread_priority_test() {
        let thread_id = thread_native_id();

        assert!(set_thread_priority(thread_id,
                                    ThreadPriority::Min,
                                    ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
        assert!(set_thread_priority(thread_id,
                                    ThreadPriority::Max,
                                    ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
        assert!(set_thread_priority(thread_id,
                                    ThreadPriority::Specific(0),
                                    ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal)).is_ok());
    }
}
