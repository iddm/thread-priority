//! This module defines the unix thread control.
//!
//! The crate's prelude doesn't have much control over
//! the unix threads, and this module provides
//! better control over those.

use std::convert::TryFrom;

#[cfg(target_os = "android")]
use libc::SCHED_NORMAL as SCHED_OTHER;
#[cfg(not(target_os = "android"))]
use libc::SCHED_OTHER;
#[cfg(any(target_os = "linux", target_os = "android"))]
use libc::{SCHED_BATCH, SCHED_IDLE};
use libc::{SCHED_FIFO, SCHED_RR};

use crate::{Error, ThreadPriority, ThreadPriorityValue};
use std::mem::MaybeUninit;

// Processes scheduled under one of the real-time policies
// (SCHED_FIFO, SCHED_RR) have a sched_priority value in the range 1
// (low) to 99 (high).
// For threads scheduled under one of the normal scheduling policies
//  (SCHED_OTHER, SCHED_IDLE, SCHED_BATCH), sched_priority is not
//  used in scheduling decisions (it must be specified as 0).
// <https://man7.org/linux/man-pages/man7/sched.7.html>

/// An alias type for a thread id.
pub type ThreadId = libc::pthread_t;

/// The maximum value possible for niceness. Threads with this value
/// of niceness have the highest priority possible
pub const NICENESS_MAX: i8 = -20;
/// The minimum value possible for niceness. Threads with this value
/// of niceness have the lowest priority possible.
pub const NICENESS_MIN: i8 = 19;

/// Proxy structure to maintain compatibility between glibc and musl
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ScheduleParams {
    /// Copy of `sched_priority` from `libc::sched_param`
    pub sched_priority: libc::c_int,
}

fn errno() -> libc::c_int {
    unsafe {
        cfg_if::cfg_if! {
            if #[cfg(any(target_os = "openbsd", target_os = "netbsd", target_os = "android"))] {
                *libc::__errno()
            } else if #[cfg(target_os = "linux")] {
                *libc::__errno_location()
            } else if #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))] {
                *libc::__error()
            } else {
                compile_error!("Your OS is probably not supported.")
            }
        }
    }
}

fn set_errno(number: libc::c_int) {
    unsafe {
        cfg_if::cfg_if! {
            if #[cfg(any(target_os = "openbsd", target_os = "netbsd", target_os = "android"))] {
                *libc::__errno() = number;
            } else if #[cfg(target_os = "linux")] {
                *libc::__errno_location() = number;
            } else if #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))] {
                *libc::__error() = number;
            } else {
                compile_error!("Your OS is probably not supported.")
            }
        }
    }
}

/// A copy of the Linux kernel's sched_attr type.
///
/// This structure can be used directly with the C api and is
/// supposed to be fully-compatible.
#[derive(Debug, Default)]
#[cfg(any(target_os = "linux", target_os = "android"))]
#[repr(C)]
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

#[cfg(any(target_os = "linux", target_os = "android"))]
bitflags::bitflags! {
    /// Flags for controlling Deadline scheduling behavior.
    #[repr(transparent)]
    pub struct DeadlineFlags: u64 {
        /// Children created by [`libc::fork`] will not inherit privileged
        /// scheduling policies.
        const RESET_ON_FORK = 0x01;
        /// The thread may reclaim bandwidth that is unused by another
        /// realtime thread.
        const RECLAIM = 0x02;
        /// Allows a task to get informed about runtime overruns through the
        /// delivery of SIGXCPU signals.
        const DEADLINE_OVERRUN = 0x04;
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl Default for DeadlineFlags {
    fn default() -> Self {
        Self::empty()
    }
}

/// Returns scheduling attributes for the current thread.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn get_thread_scheduling_attributes() -> Result<SchedAttr, Error> {
    let mut sched_attr = SchedAttr::default();
    let current_thread = 0;
    let flags = 0;
    let ret = unsafe {
        libc::syscall(
            libc::SYS_sched_getattr,
            current_thread,
            &mut sched_attr as *mut _,
            std::mem::size_of::<SchedAttr>() as u32,
            flags,
        )
    };
    if ret < 0 {
        return Err(Error::OS(errno()));
    }
    Ok(sched_attr)
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
    #[cfg(all(
        any(target_os = "linux", target_os = "android"),
        not(target_arch = "wasm32")
    ))]
    Deadline,
}

impl RealtimeThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            RealtimeThreadSchedulePolicy::Fifo => SCHED_FIFO,
            RealtimeThreadSchedulePolicy::RoundRobin => SCHED_RR,
            #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                not(target_arch = "wasm32")
            ))]
            RealtimeThreadSchedulePolicy::Deadline => 6,
        }
    }
}

/// Normal (non-realtime) schedule policies
/// For these schedule policies, [`niceness`](https://man7.org/linux/man-pages/man7/sched.7.html)
/// is used.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NormalThreadSchedulePolicy {
    /// For running very low priority background jobs.
    /// (Since Linux 2.6.23.) `SCHED_IDLE` can be used only at static priority 0;
    /// the process nice value has no influence for this policy.
    ///
    /// This policy is intended for running jobs at extremely low priority (lower even
    /// than a +19 nice value with the SCHED_OTHER or SCHED_BATCH policies).
    #[cfg(any(target_os = "linux", target_os = "android"))]
    Idle,
    /// For "batch" style execution of processes.
    /// (Since Linux 2.6.16.) `SCHED_BATCH` can be used only at static priority 0.
    /// This policy is similar to SCHED_OTHER in that it schedules the thread
    /// according to its dynamic priority (based on the nice value). The difference is
    /// that this policy will cause the scheduler to always assume that the thread is
    /// CPU-intensive. Consequently, the scheduler will apply a small scheduling penalty
    /// with respect to wakeup behavior, so that this thread is mildly disfavored in scheduling decisions.
    ///
    /// This policy is useful for workloads that are noninteractive, but do not want to lower their
    /// nice value, and for workloads that want a deterministic scheduling policy without interactivity
    /// causing extra preemptions (between the workload's tasks).
    #[cfg(any(target_os = "linux", target_os = "android"))]
    Batch,
    /// The standard round-robin time-sharing policy, also sometimes referred to as "Normal".
    ///
    /// `SCHED_OTHER` can be used at only static priority 0 (i.e., threads under real-time policies
    /// always have priority over `SCHED_OTHER` processes). `SCHED_OTHER` is the standard Linux
    /// time-sharing scheduler that is intended for all threads that do not require the special
    /// real-time mechanisms.
    ///
    /// The thread to run is chosen from the static priority 0 list based on a dynamic priority that
    /// is determined only inside this list. The dynamic  priority  is based on the nice value (see below)
    /// and is increased for each time quantum the thread is ready to run, but denied to run by the scheduler.
    ///
    /// This ensures fair progress among all `SCHED_OTHER` threads.
    ///
    /// In the Linux kernel source code, the `SCHED_OTHER` policy is actually named `SCHED_NORMAL`.
    Other,
}
impl NormalThreadSchedulePolicy {
    fn to_posix(self) -> libc::c_int {
        match self {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            NormalThreadSchedulePolicy::Idle => SCHED_IDLE,
            #[cfg(any(target_os = "linux", target_os = "android"))]
            NormalThreadSchedulePolicy::Batch => SCHED_BATCH,
            NormalThreadSchedulePolicy::Other => SCHED_OTHER,
        }
    }
}

/// Thread schedule policy definition.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ThreadSchedulePolicy {
    /// Normal thread schedule policies.
    Normal(NormalThreadSchedulePolicy),
    /// Realtime thread schedule policies.
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
            SCHED_OTHER => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Other,
            )),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            SCHED_BATCH => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Batch,
            )),
            #[cfg(any(target_os = "linux", target_os = "android"))]
            SCHED_IDLE => Ok(ThreadSchedulePolicy::Normal(
                NormalThreadSchedulePolicy::Idle,
            )),
            SCHED_FIFO => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Fifo,
            )),
            SCHED_RR => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::RoundRobin,
            )),
            #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                not(target_arch = "wasm32")
            ))]
            6 => Ok(ThreadSchedulePolicy::Realtime(
                RealtimeThreadSchedulePolicy::Deadline,
            )),
            _ => Err(Error::Ffi("Can't parse schedule policy from posix")),
        }
    }
}

impl ThreadPriority {
    /// Returns the maximum allowed value for using with the provided policy.
    /// The returned number is in the range of allowed values.
    pub fn max_value_for_policy(policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        match policy {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle) => {
                // Only `0` can be returned for `Idle` threads.
                Ok(0)
            }
            ThreadSchedulePolicy::Normal(_) => {
                // Niceness can be used, from -20 to 19, where `-20` is the maximum.
                #[cfg(any(target_os = "linux", target_os = "android"))]
                return Ok(NICENESS_MAX as libc::c_int);

                // On other systems there is no notion of using niceness
                // for just threads but for whole processes instead.
                #[cfg(not(any(target_os = "linux", target_os = "android")))]
                Err(Error::Priority(
                    "This OS doesn't support specifying this thread priority with this policy.
                    Consider changing the scheduling policy.",
                ))
            }
            _ => {
                let max_priority = unsafe { libc::sched_get_priority_max(policy.to_posix()) };
                if max_priority < 0 {
                    Err(Error::OS(errno()))
                } else {
                    Ok(max_priority)
                }
            }
        }
    }

    /// Returns the minimum allowed value for using with the provided policy.
    /// The returned number is in the range of allowed values.
    pub fn min_value_for_policy(policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        match policy {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle) => Ok(0),
            ThreadSchedulePolicy::Normal(_) => {
                // Niceness can be used, from -20 to 19, where `-20` is the maximum.
                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    Ok(NICENESS_MIN as libc::c_int)
                }
                // On other systems there is no notion of using niceness
                // for just threads but for whole processes instead.
                #[cfg(not(any(target_os = "linux", target_os = "android")))]
                {
                    Err(Error::Priority(
                        "This OS doesn't support specifying this thread priority with this policy.
                    Consider changing the scheduling policy.",
                    ))
                }
            }
            _ => {
                let min_priority = unsafe { libc::sched_get_priority_min(policy.to_posix()) };
                if min_priority < 0 {
                    Err(Error::OS(errno()))
                } else {
                    Ok(min_priority)
                }
            }
        }
    }

    /// Checks that the passed priority value is within the range of allowed values for using with the provided policy.
    pub fn to_allowed_value_for_policy(
        priority: libc::c_int,
        policy: ThreadSchedulePolicy,
    ) -> Result<libc::c_int, Error> {
        let min_priority = Self::min_value_for_policy(policy)?;
        let max_priority = Self::max_value_for_policy(policy)?;
        let (min, max) = (
            std::cmp::min(min_priority, max_priority),
            std::cmp::max(min_priority, max_priority),
        );
        let allowed_range = min..=max;
        if allowed_range.contains(&priority) {
            Ok(priority)
        } else {
            Err(Error::PriorityNotInRange(allowed_range))
        }
    }

    /// Converts the priority stored to a posix number.
    /// POSIX value can not be known without knowing the scheduling policy
    /// <https://linux.die.net/man/2/sched_get_priority_max>
    ///
    /// For threads scheduled under one of the normal scheduling policies (SCHED_OTHER, SCHED_IDLE, SCHED_BATCH), sched_priority is not used in scheduling decisions (it must be specified as 0).
    /// Source: <https://man7.org/linux/man-pages/man7/sched.7.html>
    /// Due to this restriction of normal scheduling policies and the intention of the library, the niceness is used
    /// instead for such processes.
    pub fn to_posix(self, policy: ThreadSchedulePolicy) -> Result<libc::c_int, Error> {
        let ret = match self {
            ThreadPriority::Min => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(
                    any(target_os = "linux", target_os = "android"),
                    not(target_arch = "wasm32")
                ))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                _ => Self::min_value_for_policy(policy).map(|v| v as u32),
            },
            ThreadPriority::Crossplatform(ThreadPriorityValue(p)) => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(
                    any(target_os = "linux", target_os = "android"),
                    not(target_arch = "wasm32")
                ))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                ThreadSchedulePolicy::Realtime(_) => {
                    Self::to_allowed_value_for_policy(p as i32, policy).map(|v| v as u32)
                }
                ThreadSchedulePolicy::Normal(_) => {
                    let niceness_values = NICENESS_MAX.abs() + NICENESS_MIN.abs();
                    let ratio = p as f32 / ThreadPriorityValue::MAX as f32;
                    let niceness = ((niceness_values as f32 * ratio) as i8 + NICENESS_MAX) as i32;
                    Self::to_allowed_value_for_policy(niceness, policy).map(|v| v as u32)
                }
            },
            // TODO avoid code duplication.
            ThreadPriority::Os(crate::ThreadPriorityOsValue(p)) => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(
                    any(target_os = "linux", target_os = "android"),
                    not(target_arch = "wasm32")
                ))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                _ => Self::to_allowed_value_for_policy(p as i32, policy).map(|v| v as u32),
            },
            ThreadPriority::Max => match policy {
                // SCHED_DEADLINE doesn't really have a notion of priority, this is an error
                #[cfg(all(
                    any(target_os = "linux", target_os = "android"),
                    not(target_arch = "wasm32")
                ))]
                ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => Err(
                    Error::Priority("Deadline scheduling must use deadline priority."),
                ),
                _ => Self::max_value_for_policy(policy).map(|v| v as u32),
            },
            #[cfg(all(
                any(target_os = "linux", target_os = "android"),
                not(target_arch = "wasm32")
            ))]
            ThreadPriority::Deadline { .. } => Err(Error::Priority(
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

#[cfg(any(target_os = "linux", target_os = "android"))]
fn set_thread_priority_and_policy_deadline(
    native: ThreadId,
    priority: ThreadPriority,
) -> Result<(), Error> {
    use std::convert::TryInto as _;

    let (runtime, deadline, period, flags) = match priority {
        ThreadPriority::Deadline {
            runtime,
            deadline,
            period,
            flags,
        } => (|| {
            Ok((
                runtime.as_nanos().try_into()?,
                deadline.as_nanos().try_into()?,
                period.as_nanos().try_into()?,
                flags,
            ))
        })()
        .map_err(|_: std::num::TryFromIntError| {
            Error::Priority("Deadline policy durations don't fit into a `u64`.")
        })?,
        _ => {
            return Err(Error::Priority(
                "Deadline policy given without deadline priority.",
            ))
        }
    };
    let tid = native as libc::pid_t;
    let sched_attr = SchedAttr {
        size: std::mem::size_of::<SchedAttr>() as u32,
        sched_policy: RealtimeThreadSchedulePolicy::Deadline.to_posix() as u32,
        sched_runtime: runtime,
        sched_deadline: deadline,
        sched_period: period,
        sched_flags: flags.bits(),
        ..Default::default()
    };
    let ret =
        unsafe { libc::syscall(libc::SYS_sched_setattr, tid, &sched_attr as *const _, 0) as i32 };

    match ret {
        0 => Ok(()),
        e => Err(Error::OS(e)),
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
///
/// # Note
///
/// In case the value is specified as [`ThreadPriority::Crossplatform`] and is incompatible with the policy, an error is returned.
/// However if [`ThreadPriority::Min`] or [`ThreadPriority::Max`] are used, the correct value is used automatically according
/// to the range of the policy's allowed values.
pub fn set_thread_priority_and_policy(
    native: ThreadId,
    priority: ThreadPriority,
    policy: ThreadSchedulePolicy,
) -> Result<(), Error> {
    match policy {
        // SCHED_DEADLINE policy requires its own syscall
        #[cfg(any(target_os = "linux", target_os = "android"))]
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) => {
            set_thread_priority_and_policy_deadline(native, priority)
        }
        _ => {
            let fixed_priority = priority.to_posix(policy)?;
            // On macOS and iOS it is possible to set the priority
            // this way.
            if matches!(policy, ThreadSchedulePolicy::Realtime(_))
                || cfg!(any(target_os = "macos", target_os = "ios"))
            {
                // If the policy is a realtime one, the priority is set via
                // pthread_setschedparam.
                let params = ScheduleParams {
                    sched_priority: fixed_priority,
                }
                .into_posix();

                let ret = unsafe {
                    libc::pthread_setschedparam(
                        native,
                        policy.to_posix(),
                        &params as *const libc::sched_param,
                    )
                };

                match ret {
                    0 => Ok(()),
                    e => Err(Error::OS(e)),
                }
            } else {
                // If this is a normal-scheduled thread, the priority is
                // set via niceness.
                set_errno(0);

                let ret = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, fixed_priority) };
                if ret == 0 {
                    return Ok(());
                }

                match errno() {
                    0 => Ok(()),
                    e => Err(Error::OS(e)),
                }
            }
        }
    }
}

/// Set current thread's priority.
/// In order to properly map a value of the thread priority, the thread scheduling
/// must be known. This function attempts to retrieve the current thread's
/// scheduling policy and thus map the priority value correctly, so that it fits
/// within the scheduling policy's allowed range of values.
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
    let policy = thread_schedule_policy()?;
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
    /// For more info read [`set_thread_priority_and_policy`].
    fn set_priority_and_policy(
        &self,
        policy: ThreadSchedulePolicy,
        priority: ThreadPriority,
    ) -> Result<(), Error> {
        cfg_if::cfg_if! {
            if #[cfg(all(any(target_os = "linux", target_os = "android"), not(target_arch = "wasm32")))] {
                if policy == ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline) {
                    set_thread_priority_and_policy(thread_native_id(), ThreadPriority::Crossplatform(ThreadPriorityValue(0)), policy)
                } else {
                    set_thread_priority_and_policy(thread_native_id(), priority, policy)
                }
            } else {
                set_thread_priority_and_policy(thread_native_id(), priority, policy)
            }
        }
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
        use std::time::Duration;

        assert!(set_thread_priority_and_policy(
            0, // current thread
            ThreadPriority::Deadline {
                runtime: Duration::from_millis(1),
                deadline: Duration::from_millis(10),
                period: Duration::from_millis(100),
                flags: DeadlineFlags::RESET_ON_FORK,
            },
            ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Deadline)
        )
        .is_ok());

        let attributes = get_thread_scheduling_attributes().unwrap();
        assert_eq!(
            attributes.sched_policy,
            RealtimeThreadSchedulePolicy::Deadline.to_posix() as u32
        );
        assert_eq!(attributes.sched_runtime, 1 * 10_u64.pow(6));
        assert_eq!(attributes.sched_deadline, 10 * 10_u64.pow(6));
        assert_eq!(attributes.sched_period, 100 * 10_u64.pow(6));
        assert_eq!(attributes.sched_flags, DeadlineFlags::RESET_ON_FORK.bits());
    }
}
