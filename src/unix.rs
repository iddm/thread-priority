//! This module defines the unix thread control.
//!
//! The crate's prelude doesn't have much control over
//! the unix threads, and this module provides
//! better control over those.

use crate::{Error, ThreadPriority};

/// An alias type for a thread id.
pub type ThreadId = libc::pthread_t;

/// Proxy structure to maintain compatibility between glibc and musl
pub struct ScheduleParams {
    /// Copy of `sched_priority` from `libc::sched_param`
    pub sched_priority: libc::c_int,
}

/// Copy of the Linux kernel's sched_attr type
#[repr(C)]
#[derive(Debug, Default)]
#[cfg(target_os = "linux")]
pub struct SchedAttr {
    size: u32,
    sched_policy: u32,
    sched_flags: u64,

    /// for SCHED_NORMAL and SCHED_BATCH
    sched_nice: i32,
    /// for SCHED_FIFO, SCHED_RR
    sched_priority: u32,

    /// for SCHED_DEADLINE
    sched_runtime: u64,
    /// for SCHED_DEADLINE
    sched_deadline: u64,
    /// for SCHED_DEADLINE
    sched_period: u64,

    /// Utilization hint
    sched_util_min: u32,
    /// Utilization hint
    sched_util_max: u32,
}

impl ScheduleParams {
    #[cfg(not(target_env = "musl"))]
    fn into_posix(self) -> libc::sched_param {
        libc::sched_param {
            sched_priority: self.sched_priority,
        }
    }

    #[cfg(target_env = "musl")]
    fn into_posix(self) -> libc::sched_param {
        use libc::timespec as TimeSpec;

        libc::sched_param {
            sched_priority: self.sched_priority,
            sched_ss_low_priority: 0,
            sched_ss_repl_period: TimeSpec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            sched_ss_init_budget: TimeSpec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            sched_ss_max_repl: 0,
        }
    }

    fn from_posix(sched_param: libc::sched_param) -> Self {
        ScheduleParams {
            sched_priority: sched_param.sched_priority,
        }
    }
}

/// The following "real-time" policies are also supported, for special time-critical applications
/// that need precise control over the way in which runnable processes are selected for execution
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum RealtimeThreadSchedulePolicy {
    /// A first-in, first-out policy
    Fifo,
    /// A round-robin policy
    RoundRobin,
    /// A deadline policy. Note, due to Linux expecting a pid_t and not a pthread_t, the given
    /// [ThreadId](struct.ThreadId) will be interpreted as a pid_t. This policy is NOT
    /// POSIX-compatible, so we only include it for linux targets.
    #[cfg(target_os = "linux")]
    Deadline,
}
impl RealtimeThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            RealtimeThreadSchedulePolicy::Fifo => 1,
            RealtimeThreadSchedulePolicy::RoundRobin => 2,
            #[cfg(target_os = "linux")]
            RealtimeThreadSchedulePolicy::Deadline => 6,
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
            #[cfg(target_os = "linux")]
            6 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Deadline,
            )),
            _ => Err(Error::Ffi("Can't parse schedule policy from posix")),
        }
    }
}

impl ThreadPriority {
    /// POSIX value can not be known without knowing the scheduling policy
    /// <https://linux.die.net/man/2/sched_get_priority_max>
    pub fn to_posix(self, policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        let ret = match self {
            ThreadPriority::Min => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority,
                // so fix min and max time slices to 100ms (the syscall takes ns).
                #[cfg(target_os = "linux")]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => {
                    Ok(100 * 10_u32.pow(6))
                }
                ThreadSchedulePolicy::Realtime(_) => Ok(1),
                _ => Ok(0),
            },
            ThreadPriority::Specific(p) => match policy {
                // SCHED_DEADLINE priorities are nanoseconds for runtime, deadline, and period,
                // accept any value
                #[cfg(target_os = "linux")]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Ok(p),
                ThreadSchedulePolicy::Realtime(_) if (p == 0 || p > 99) => {
                    Err(Error::Priority("The value is out of range [0; 99]"))
                }
                ThreadSchedulePolicy::Normal(_) if p != 0 => Err(Error::Priority(
                    "The value can be only 0 for normal scheduling policy",
                )),
                _ => Ok(p),
            },
            ThreadPriority::Max => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority,
                // so fix min and max time slices to 100ms (the syscall takes ns).
                #[cfg(target_os = "linux")]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => {
                    Ok(100 * 10_u32.pow(6))
                }
                ThreadSchedulePolicy::Realtime(_) => Ok(99),
                _ => Ok(0),
            },
        };
        ret.map(|p| p as libc::c_int)
    }

    /// Gets priority value from POSIX value.
    /// In order to interpret it correctly, you should also take scheduling policy
    /// into account.
    pub fn from_posix(params: ScheduleParams) -> ThreadPriority {
        ThreadPriority::Specific(params.sched_priority as u32)
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
    native: ThreadId,
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
/// * Deadline policy requires a tid, not a pthread_t, so invoking this while using a deadline
/// policy will interpret the given [ThreadId](struct.ThreadId) as a pid_t (thread tid).
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
    native: ThreadId,
    policy: ThreadSchedulePolicy,
    params: ScheduleParams,
) -> Result<(), Error> {
    let params = params.into_posix();
    unsafe {
        let ret = match policy {
            // SCHED_DEADLINE policy requires its own syscall
            #[cfg(target_os = "linux")]
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => {
                let tid = native as libc::pid_t;
                let sched_attr = SchedAttr {
                    size: std::mem::size_of::<SchedAttr>() as u32,
                    sched_policy: policy.to_posix() as u32,

                    sched_runtime: params.sched_priority as u64,
                    sched_deadline: params.sched_priority as u64,
                    sched_period: params.sched_priority as u64,

                    ..Default::default()
                };
                libc::syscall(
                    libc::SYS_sched_setattr,
                    tid,
                    &sched_attr as *const _,
                    // we are not setting SCHED_FLAG_RECLAIM nor SCHED_FLAG_DL_OVERRUN
                    0,
                ) as i32
            }
            _ => libc::pthread_setschedparam(
                native,
                policy.to_posix(),
                &params as *const libc::sched_param,
            ),
        };
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
    native: ThreadId,
) -> Result<(ThreadSchedulePolicy, ScheduleParams), Error> {
    unsafe {
        let mut policy = 0i32;
        let mut params = ScheduleParams { sched_priority: 0 }.into_posix();

        let ret = libc::pthread_getschedparam(
            native,
            &mut policy as *mut libc::c_int,
            &mut params as *mut libc::sched_param,
        );
        match ret {
            0 => Ok((
                ThreadSchedulePolicy::from_posix(policy)?,
                ScheduleParams::from_posix(params),
            )),
            e => Err(Error::OS(e)),
        }
    }
}

/// Get current thread's priority value.
pub fn thread_priority() -> Result<ThreadPriority, Error> {
    Ok(ThreadPriority::from_posix(
        thread_schedule_policy_param(thread_native_id())?.1,
    ))
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
pub fn thread_native_id() -> ThreadId {
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
