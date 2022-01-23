//! This module defines the unix thread control.
//!
//! The crate's prelude doesn't have much control over
//! the unix threads, and this module provides
//! better control over those.

use std::convert::TryFrom;

use const_format::concatcp;

use crate::{Error, ThreadPriority, ThreadPriorityValue};
use std::mem::MaybeUninit;

#[cfg(not(target_os = "macos"))]
const SCHED_FIFO: i32 = 1;
#[cfg(target_os = "macos")]
const SCHED_FIFO: i32 = 4;

// Processes scheduled under one of the real-time policies
// (SCHED_FIFO, SCHED_RR) have a sched_priority value in the range 1
// (low) to 99 (high).
const MIN_PRIORITY: i32 = 1;
const MAX_PRIORITY: i32 = 99;
// For threads scheduled under one of the normal scheduling policies
//  (SCHED_OTHER, SCHED_IDLE, SCHED_BATCH), sched_priority is not
//  used in scheduling decisions (it must be specified as 0).
// <https://man7.org/linux/man-pages/man7/sched.7.html>
const NORMAL_PRIORITY: i32 = 0;

/// An alias type for a thread id.
pub type ThreadId = libc::pthread_t;

/// Proxy structure to maintain compatibility between glibc and musl
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
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
    fn into_posix(self) -> libc::sched_param {
        let mut param = unsafe { MaybeUninit::<libc::sched_param>::zeroed().assume_init() };
        param.sched_priority = self.sched_priority;
        param
    }

    fn from_posix(sched_param: libc::sched_param) -> Self {
        ScheduleParams {
            sched_priority: sched_param.sched_priority,
        }
    }
}

/// The following "real-time" policies are also supported, for special time-critical applications
/// that need precise control over the way in which runnable processes are selected for execution
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum RealtimeThreadSchedulePolicy {
    /// A first-in, first-out policy
    Fifo,
    /// A round-robin policy
    RoundRobin,
    /// A deadline policy. Note, due to Linux expecting a pid_t and not a pthread_t, the given
    /// [ThreadId](struct.ThreadId) will be interpreted as a pid_t. This policy is NOT
    /// POSIX-compatible, so we only include it for linux targets.
    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    Deadline,
}

impl RealtimeThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            RealtimeThreadSchedulePolicy::Fifo => SCHED_FIFO,
            RealtimeThreadSchedulePolicy::RoundRobin => 2,
            #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
            RealtimeThreadSchedulePolicy::Deadline => 6,
        }
    }
}

/// Normal (usual) schedule policies
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NormalThreadSchedulePolicy {
    /// For running very low priority background jobs
    #[cfg(not(target_os = "macos"))]
    Idle,
    /// For "batch" style execution of processes
    #[cfg(not(target_os = "macos"))]
    Batch,
    /// The standard round-robin time-sharing policy
    Other,
    /// The standard round-robin time-sharing policy
    #[cfg(not(target_os = "macos"))]
    Normal,
}
impl NormalThreadSchedulePolicy {
    #[cfg(not(target_os = "macos"))]
    fn to_posix(self) -> libc::c_int {
        match self {
            NormalThreadSchedulePolicy::Idle => 5,
            NormalThreadSchedulePolicy::Batch => 3,
            NormalThreadSchedulePolicy::Other | NormalThreadSchedulePolicy::Normal => 0,
        }
    }

    #[cfg(target_os = "macos")]
    fn to_posix(self) -> libc::c_int {
        match self {
            NormalThreadSchedulePolicy::Other => 1,
        }
    }
}

/// Thread schedule policy definition
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
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

    #[cfg(not(target_os = "macos"))]
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
            SCHED_FIFO => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Fifo,
            )),
            2 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::RoundRobin,
            )),
            #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
            6 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Deadline,
            )),
            _ => Err(Error::Ffi("Can't parse schedule policy from posix")),
        }
    }

    #[cfg(target_os = "macos")]
    fn from_posix(policy: libc::c_int) -> Result<ThreadSchedulePolicy, Error> {
        match policy {
            1 => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Other,
            )),
            SCHED_FIFO => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Fifo,
            )),
            2 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::RoundRobin,
            )),
            _ => Err(Error::Ffi(
                "Can't parse schedule policy from berkley values",
            )),
        }
    }
}

impl ThreadPriority {
    /// POSIX value can not be known without knowing the scheduling policy
    /// <https://linux.die.net/man/2/sched_get_priority_max>
    pub fn to_posix(self, policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        let ret = match self {
            ThreadPriority::Min => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                ThreadSchedulePolicy::Realtime(_) => Ok(MIN_PRIORITY as u32),
                _ => Err(Error::Priority(concatcp!(
                    "The non-realtime schedule policies can't have priority higher than ",
                    NORMAL_PRIORITY
                ))),
            },
            ThreadPriority::Crossplatform(ThreadPriorityValue(p)) => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                ThreadSchedulePolicy::Realtime(_)
                    if !((MIN_PRIORITY..=MAX_PRIORITY).contains(&(p as i32))) =>
                {
                    Err(Error::Priority(concatcp!(
                        "The value is out of range [",
                        MIN_PRIORITY,
                        "; ",
                        MAX_PRIORITY,
                        "]"
                    )))
                }
                ThreadSchedulePolicy::Normal(_) if p as i32 != NORMAL_PRIORITY => Err(
                    Error::Priority("The value can be only 0 for normal scheduling policy"),
                ),
                _ => Ok(p as u32),
            },
            // TODO avoid code duplication.
            ThreadPriority::Os(crate::ThreadPriorityOsValue(p)) => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                ThreadSchedulePolicy::Realtime(_)
                    if !((MIN_PRIORITY..=MAX_PRIORITY).contains(&(p as i32))) =>
                {
                    Err(Error::Priority(concatcp!(
                        "The value is out of range [",
                        MIN_PRIORITY,
                        "; ",
                        MAX_PRIORITY,
                        "]"
                    )))
                }
                ThreadSchedulePolicy::Normal(_) if p as i32 != NORMAL_PRIORITY => {
                    Err(Error::Priority(concatcp!(
                        "The value can be only ",
                        NORMAL_PRIORITY,
                        " for normal scheduling policy"
                    )))
                }
                _ => Ok(p),
            },
            ThreadPriority::Max => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                ThreadSchedulePolicy::Realtime(_) => Ok(MAX_PRIORITY as u32),
                _ => Err(Error::Priority(concatcp!(
                    "The non-realtime schedule policies can't have priority higher than ",
                    NORMAL_PRIORITY
                ))),
            },
            #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
            ThreadPriority::Deadline(_, _, _) => Err(Error::Priority(
                "Deadline is non-POSIX and cannot be converted.",
            )),
        };
        ret.map(|p| p as libc::c_int)
    }

    /// Gets priority value from POSIX value.
    /// In order to interpret it correctly, you should also take scheduling policy
    /// into account.
    pub fn from_posix(params: ScheduleParams) -> ThreadPriority {
        ThreadPriority::Crossplatform(ThreadPriorityValue(params.sched_priority as u8))
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
///                                        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo)).is_ok());
/// ```
pub fn set_thread_priority_and_policy(
    native: ThreadId,
    priority: ThreadPriority,
    policy: ThreadSchedulePolicy,
) -> Result<(), Error> {
    let params = ScheduleParams {
        sched_priority: match policy {
            #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => 0,
            _ => priority.to_posix(policy)?,
        },
    };
    set_thread_schedule_policy(native, policy, params, priority)
}

/// Set current thread's priority.
/// To make the priority of use, the scheduling policy is determined from the value.
/// If the value is not equal to `ThreadPriority::Crossplatform(0)`, then the one of
/// the available real-time policies is used. Otherwise, the function does nothing,
/// as the only reasonable outcome with zero-priority set would be to change a scheduling
/// policy, for which there is [`set_thread_priority_and_policy`].
///
/// * May require privileges
///
/// ```rust
/// use thread_priority::*;
///
/// let thread_id = thread_native_id();
/// assert!(set_current_thread_priority(ThreadPriority::Min).is_ok());
/// ```
pub fn set_current_thread_priority(priority: ThreadPriority) -> Result<(), Error> {
    let thread_id = thread_native_id();
    let policy = ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo);
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
    thread_schedule_policy_param(thread_native_id()).map(|policy| policy.0)
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
/// let priority = ThreadPriority::Min;
/// assert!(set_thread_schedule_policy(thread_id, policy, params, priority).is_ok());
/// ```
pub fn set_thread_schedule_policy(
    native: ThreadId,
    policy: ThreadSchedulePolicy,
    params: ScheduleParams,
    priority: ThreadPriority,
) -> Result<(), Error> {
    let params = params.into_posix();
    unsafe {
        let ret = match policy {
            // SCHED_DEADLINE policy requires its own syscall
            #[cfg(target_os = "linux")]
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => {
                let (runtime, deadline, period) = match priority {
                    ThreadPriority::Deadline(r, d, p) => (r, d, p),
                    _ => {
                        return Err(Error::Priority(
                            "Deadline policy given without deadline priority.",
                        ))
                    }
                };
                let tid = native as libc::pid_t;
                let sched_attr = SchedAttr {
                    size: std::mem::size_of::<SchedAttr>() as u32,
                    sched_policy: policy.to_posix() as u32,

                    sched_runtime: runtime as u64,
                    sched_deadline: deadline as u64,
                    sched_period: period as u64,

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
        // This is just to silent the unused variable warning.
        let _priority = priority;
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

/// Get the thread's priority value.
pub fn get_thread_priority(native: ThreadId) -> Result<ThreadPriority, Error> {
    Ok(ThreadPriority::from_posix(
        thread_schedule_policy_param(native)?.1,
    ))
}

/// Get current thread's priority value.
pub fn get_current_thread_priority() -> Result<ThreadPriority, Error> {
    get_thread_priority(thread_native_id())
}

/// A helper trait for other threads to implement to be able to call methods
/// on threads themselves.
///
/// ```rust
/// use thread_priority::*;
///
/// assert!(std::thread::current().get_priority().is_ok());
///
/// let join_handle = std::thread::spawn(|| println!("Hello world!"));
/// assert!(join_handle.thread().get_priority().is_ok());
///
/// join_handle.join();
/// ```
pub trait ThreadExt {
    /// Gets the current thread's priority.
    /// For more info read [`get_current_thread_priority`].
    ///
    /// ```rust
    /// use thread_priority::*;
    ///
    /// assert!(std::thread::current().get_priority().is_ok());
    /// ```
    fn get_priority(&self) -> Result<ThreadPriority, Error> {
        get_current_thread_priority()
    }

    /// Sets the current thread's priority.
    /// For more info see [`ThreadPriority::set_for_current`].
    ///
    /// ```rust
    /// use thread_priority::*;
    ///
    /// assert!(std::thread::current().set_priority(ThreadPriority::Min).is_ok());
    /// ```
    fn set_priority(&self, priority: ThreadPriority) -> Result<(), Error> {
        priority.set_for_current()
    }

    /// Gets the current thread's schedule policy.
    /// For more info read [`thread_schedule_policy`].
    fn get_schedule_policy(&self) -> Result<ThreadSchedulePolicy, Error> {
        thread_schedule_policy()
    }

    /// Returns current thread's schedule policy and parameters.
    /// For more info read [`thread_schedule_policy_param`].
    fn get_schedule_policy_param(&self) -> Result<(ThreadSchedulePolicy, ScheduleParams), Error> {
        thread_schedule_policy_param(thread_native_id())
    }

    /// Sets current thread's schedule policy.
    /// For more info read [`set_thread_schedule_policy`].
    fn set_schedule_policy(
        &self,
        policy: ThreadSchedulePolicy,
        priority: ThreadPriority,
    ) -> Result<(), Error> {
        let params = ScheduleParams {
            sched_priority: match policy {
                #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => 0,
                _ => priority.to_posix(policy)?,
            },
        };
        set_thread_schedule_policy(thread_native_id(), policy, params, priority)
    }

    /// Returns native unix thread id.
    /// For more info read [`thread_native_id`].
    ///
    /// ```rust
    /// use thread_priority::*;
    ///
    /// assert!(std::thread::current().get_native_id() > 0);
    fn get_native_id(&self) -> ThreadId {
        thread_native_id()
    }
}

/// Auto-implementation of this trait for the [`std::thread::Thread`].
impl ThreadExt for std::thread::Thread {}

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

impl TryFrom<u8> for ThreadPriority {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if let 0..=100 = value {
            Ok(ThreadPriority::Crossplatform(ThreadPriorityValue(value)))
        } else {
            Err("The thread priority value must be in range of [0; 100].")
        }
    }
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
    #[cfg(target_os = "linux")]
    fn set_deadline_policy() {
        // allow the identity operation for clarity
        #![allow(clippy::identity_op)]

        assert!(set_thread_priority_and_policy(
            0, // current thread
            ThreadPriority::Deadline(1 * 10_u64.pow(6), 10 * 10_u64.pow(6), 100 * 10_u64.pow(6)),
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline)
        )
        .is_ok());

        // now we check the return values
        unsafe {
            let mut sched_attr = SchedAttr::default();
            let ret = libc::syscall(
                libc::SYS_sched_getattr,
                0, // current thread
                &mut sched_attr as *mut _,
                std::mem::size_of::<SchedAttr>() as u32,
                0, // flags must be 0
            );

            assert!(ret >= 0);
            assert_eq!(
                sched_attr.sched_policy,
                RealtimeThreadSchedulePolicy::Deadline.to_posix() as u32
            );
            assert_eq!(sched_attr.sched_runtime, 1 * 10_u64.pow(6));
            assert_eq!(sched_attr.sched_deadline, 10 * 10_u64.pow(6));
            assert_eq!(sched_attr.sched_period, 100 * 10_u64.pow(6));
        }
    }
}
