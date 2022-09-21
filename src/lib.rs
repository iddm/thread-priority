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
//!
//! # More examples
//!
//! ### Minimal cross-platform examples
//! Setting current thread's priority to minimum:
//!
//! ```rust,no_run
//! use thread_priority::*;
//!
//! assert!(set_current_thread_priority(ThreadPriority::Min).is_ok());
//! ```
//!
//! The same as above but using a specific value:
//!
//! ```rust,no_run
//! use thread_priority::*;
//! use std::convert::TryInto;
//!
//! // The lower the number the lower the priority.
//! assert!(set_current_thread_priority(ThreadPriority::Crossplatform(0.try_into().unwrap())).is_ok());
//! ```
//!
//! ### Building a thread using the [`ThreadBuilderExt`] trait
//!
//! ```rust,no_run
//! use thread_priority::*;
//! use thread_priority::ThreadBuilderExt;
//!
//! let thread = std::thread::Builder::new()
//!     .name("MyNewThread".to_owned())
//!     .spawn_with_priority(ThreadPriority::Max, |result| {
//!         // This is printed out from within the spawned thread.
//!         println!("Set priority result: {:?}", result);
//!         assert!(result.is_ok());
//! }).unwrap();
//! thread.join();
//! ```
//!
//! ### Building a thread using the [`ThreadBuilder`].
//!
//! ```rust,no_run
//! use thread_priority::*;
//!
//! let thread = ThreadBuilder::default()
//!     .name("MyThread")
//!     .priority(ThreadPriority::Max)
//!     .spawn(|result| {
//!         // This is printed out from within the spawned thread.
//!         println!("Set priority result: {:?}", result);
//!         assert!(result.is_ok());
//! }).unwrap();
//! thread.join();
//!
//! // Another example where we don't care about the priority having been set.
//! let thread = ThreadBuilder::default()
//!     .name("MyThread")
//!     .priority(ThreadPriority::Max)
//!     .spawn_careless(|| {
//!         // This is printed out from within the spawned thread.
//!         println!("We don't care about the priority result.");
//! }).unwrap();
//! thread.join();
//! ```
//!
//! ### Using [`ThreadExt`] trait on the current thread
//!
//! ```rust,no_run
//! use thread_priority::*;
//!
//! assert!(std::thread::current().get_priority().is_ok());
//! println!("This thread's native id is: {:?}", std::thread::current().get_native_id());
//! ```
//!
#![warn(missing_docs)]
#![deny(warnings)]

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "android",
    target_arch = "wasm32",
))]
pub mod unix;
#[cfg(any(target_os = "linux", target_os = "android"))]
use std::time::Duration;

#[cfg(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "android",
    target_arch = "wasm32",
))]
pub use unix::*;

#[cfg(windows)]
pub mod windows;
#[cfg(windows)]
pub use windows::*;

/// A error type
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Error {
    /// A value which describes why it is impossible to use such a priority.
    Priority(&'static str),
    /// Indicates that the priority isn't in range and it should be within the provided range.
    /// This may happen on different operating systems following a single standard of API but
    /// allowing different priority values for different scheduling policies.
    PriorityNotInRange(std::ops::RangeInclusive<i32>),
    /// Target OS' error type. In most systems it is an integer which
    /// later should be used with target OS' API for understanding the value.
    /// On Linux there is an integer containing an error code from errno.
    /// For Windows it contains a number used in Windows for the same purpose.
    OS(i32),
    /// FFI failure.
    Ffi(&'static str),
}

/// Platform-independent thread priority value.
/// Should be in `[0; 100)` range. The higher the number is - the higher
/// the priority.
///
/// The only way to create such a value is a safe conversion from an 8-byte
/// unsigned integer ([`u8`]):
///
/// ```rust
/// use thread_priority::*;
/// use std::convert::{TryFrom, TryInto};
///
/// // Create the lowest possible priority value.
/// assert!(ThreadPriorityValue::try_from(0u8).is_ok());
/// // Create it implicitly via `TryInto`:
/// let _priority = ThreadPriority::Crossplatform(0u8.try_into().unwrap());
/// ```
///
/// In case you need to get the raw value out of it, use the `Into<u8>` trait:
///
/// ```rust
/// use thread_priority::*;
/// use std::convert::TryFrom;
///
/// // Create the lowest possible priority value.
/// let priority = ThreadPriorityValue::try_from(0u8).unwrap();
/// // Create it implicitly via `TryInto`:
/// let raw_value: u8 = priority.into();
/// assert_eq!(raw_value, 0);
/// ```
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ThreadPriorityValue(u8);
impl ThreadPriorityValue {
    /// The maximum value for a thread priority.
    pub const MAX: u8 = 99;
    /// The minimum value for a thread priority.
    pub const MIN: u8 = 0;
}

impl std::convert::TryFrom<u8> for ThreadPriorityValue {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err("The value is not in the range of [0;99]")
        }
    }
}

// The From<u8> is unsafe, so there is a TryFrom instead.
// For this reason we silent the warning from clippy.
#[allow(clippy::from_over_into)]
impl std::convert::Into<u8> for ThreadPriorityValue {
    fn into(self) -> u8 {
        self.0
    }
}

/// Platform-specific thread priority value.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ThreadPriorityOsValue(u32);

/// Thread priority enumeration.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum ThreadPriority {
    /// Holds a value representing the minimum possible priority.
    #[cfg_attr(
        target_os = "windows",
        doc = "\
The [`ThreadPriority::Min`] value is mapped to [`WinAPIThreadPriority::Lowest`] and not
[`WinAPIThreadPriority::Idle`] to avoid unexpected drawbacks. Use the specific value
to set it to [`WinAPIThreadPriority::Idle`] when it is really needed.
"
    )]
    Min,
    /// Holds a platform-independent priority value.
    /// Usually used when setting a value, for sometimes it is not possible to map
    /// the operating system's priority to this value.
    Crossplatform(ThreadPriorityValue),
    /// Holds an operating system specific value. If it is not possible to obtain the
    /// [`ThreadPriority::Crossplatform`] variant of the value, this is returned instead.
    #[cfg_attr(
        target_os = "windows",
        doc = "\
The value is matched among possible values in Windows from [`WinAPIThreadPriority::Idle`] till
[`WinAPIThreadPriority::TimeCritical`]. This is due to windows only having from 7 to 9 possible
thread priorities and not `100` as it is allowed to have in the [`ThreadPriority::Crossplatform`]
variant.
"
    )]
    Os(ThreadPriorityOsValue),
    /// Holds scheduling parameters for Deadline scheduling. These are, in order,
    /// the nanoseconds for runtime, deadline, and period. Please note that the
    /// kernel enforces runtime <= deadline <= period.
    ///
    ///   arrival/wakeup                    absolute deadline
    ///        |    start time                    |
    ///        |        |                         |
    ///        v        v                         v
    ///   -----x--------xooooooooooooooooo--------x--------x---
    ///                 |<-- Runtime ------->|
    ///        |<----------- Deadline ----------->|
    ///        |<-------------- Period ------------------->|
    #[cfg(any(target_os = "linux", target_os = "android"))]
    Deadline {
        /// Set this to something larger than the average computation time
        /// or to the worst-case computation time for hard real-time tasks.
        runtime: Duration,
        /// Set this to the relative deadline.
        deadline: Duration,
        /// Set this to the period of the task.
        period: Duration,
    },
    /// Holds a value representing the maximum possible priority.
    /// Should be used with caution, it solely depends on the target
    /// os where the program is going to be running on, how it will
    /// behave. On some systems, the whole system may become frozen
    /// if not used properly.
    #[cfg_attr(
        target_os = "windows",
        doc = "\
The [`ThreadPriority::Max`] value is mapped to [`WinAPIThreadPriority::Highest`] and not
[`WinAPIThreadPriority::TimeCritical`] to avoid unexpected drawbacks. Use the specific value
to set it to [`WinAPIThreadPriority::TimeCritical`] when it is really needed.
"
    )]
    Max,
}

impl ThreadPriority {
    /// Sets current thread's priority to this value.
    pub fn set_for_current(self) -> Result<(), Error> {
        set_current_thread_priority(self)
    }
}

/// Represents an OS thread.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Thread {
    /// Thread's priority.
    pub priority: ThreadPriority,
    /// Thread's ID (or handle).
    pub id: ThreadId,
}

impl Thread {
    /// Get current thread.
    ///
    /// # Usage
    ///
    /// ```rust
    /// use thread_priority::*;
    ///
    /// assert!(Thread::current().is_ok());
    /// ```
    pub fn current() -> Result<Thread, Error> {
        Ok(Thread {
            priority: get_current_thread_priority()?,
            id: thread_native_id(),
        })
    }
}

/// A copy of the [`std::thread::Builder`] builder allowing to set priority settings.
///
/// ```rust
/// use thread_priority::*;
///
/// let thread = ThreadBuilder::default()
///     .name("MyThread")
///     .priority(ThreadPriority::Max)
///     .spawn(|result| {
///         // This is printed out from within the spawned thread.
///         println!("Set priority result: {:?}", result);
///         assert!(result.is_ok());
/// }).unwrap();
/// thread.join();
///
/// // Another example where we don't care about the priority having been set.
/// let thread = ThreadBuilder::default()
///     .name("MyThread")
///     .priority(ThreadPriority::Max)
///     .spawn_careless(|| {
///         // This is printed out from within the spawned thread.
///         println!("We don't care about the priority result.");
/// }).unwrap();
/// thread.join();
/// ```
#[derive(Clone, Debug, Default, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct ThreadBuilder {
    name: Option<String>,
    stack_size: Option<usize>,
    priority: Option<ThreadPriority>,

    #[cfg(unix)]
    policy: Option<ThreadSchedulePolicy>,

    #[cfg(windows)]
    winapi_priority: Option<WinAPIThreadPriority>,
    #[cfg(windows)]
    boost_enabled: bool,
    #[cfg(windows)]
    ideal_processor: Option<IdealProcessor>,
}

impl ThreadBuilder {
    /// Names the thread-to-be. Currently the name is used for identification
    /// only in panic messages.
    ///
    /// The name must not contain null bytes (`\0`).
    ///
    /// For more information about named threads, see
    /// [`std::thread::Builder::name()`].
    pub fn name<VALUE: Into<String>>(mut self, value: VALUE) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets the size of the stack (in bytes) for the new thread.
    ///
    /// The actual stack size may be greater than this value if
    /// the platform specifies a minimal stack size.
    ///
    /// For more information about the stack size for threads, see
    /// [`std::thread::Builder::stack_size()`].
    pub fn stack_size<VALUE: Into<usize>>(mut self, value: VALUE) -> Self {
        self.stack_size = Some(value.into());
        self
    }

    /// The thread's custom priority.
    ///
    /// For more information about the stack size for threads, see
    /// [`ThreadPriority`].
    pub fn priority<VALUE: Into<ThreadPriority>>(mut self, value: VALUE) -> Self {
        self.priority = Some(value.into());
        self
    }

    /// The thread's unix scheduling policy.
    ///
    /// For more information, see
    /// [`crate::unix::ThreadSchedulePolicy`] and [`crate::unix::set_thread_priority_and_policy`].
    #[cfg(unix)]
    pub fn policy<VALUE: Into<unix::ThreadSchedulePolicy>>(mut self, value: VALUE) -> Self {
        self.policy = Some(value.into());
        self
    }

    /// The WinAPI priority representation.
    ///
    /// For more information, see
    /// [`crate::windows::WinAPIThreadPriority`].
    #[cfg(windows)]
    pub fn winapi_priority<VALUE: Into<windows::WinAPIThreadPriority>>(
        mut self,
        value: VALUE,
    ) -> Self {
        self.winapi_priority = Some(value.into());
        self
    }

    /// Disables or enables the ability of the system to temporarily boost the priority of a thread.
    ///
    /// For more information, see
    /// [`crate::windows::set_thread_priority_boost`].
    #[cfg(windows)]
    pub fn boost_enabled(mut self, value: bool) -> Self {
        self.boost_enabled = value;
        self
    }

    /// Sets a preferred processor for a thread. The system schedules threads on their preferred
    /// processors whenever possible.
    ///
    /// For more information, see
    /// [`crate::windows::set_thread_ideal_processor`].
    #[cfg(windows)]
    pub fn ideal_processor<VALUE: Into<windows::IdealProcessor>>(mut self, value: VALUE) -> Self {
        self.ideal_processor = Some(value.into());
        self
    }

    /// Spawns a new thread by taking ownership of the `Builder`, and returns an
    /// [`std::io::Result`] to its [`std::thread::JoinHandle`].
    ///
    /// See [`std::thread::Builder::spawn`]
    #[cfg(unix)]
    pub fn spawn<F, T>(mut self, f: F) -> std::io::Result<std::thread::JoinHandle<T>>
    where
        F: FnOnce(Result<(), Error>) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let priority = self.priority;
        let policy = self.policy;

        self.build_std().spawn(move || match (priority, policy) {
            (Some(priority), Some(policy)) => f(set_thread_priority_and_policy(
                thread_native_id(),
                priority,
                policy,
            )),
            (Some(priority), None) => f(priority.set_for_current()),
            (None, Some(_policy)) => {
                unimplemented!("Setting the policy separately isn't currently supported.");
            }
            _ => f(Ok(())),
        })
    }

    /// Spawns a new thread by taking ownership of the `Builder`, and returns an
    /// [`std::io::Result`] to its [`std::thread::JoinHandle`].
    ///
    /// See [`std::thread::Builder::spawn`]
    #[cfg(windows)]
    pub fn spawn<F, T>(mut self, f: F) -> std::io::Result<std::thread::JoinHandle<T>>
    where
        F: FnOnce(Result<(), Error>) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        let thread_priority = self.priority;
        let winapi_priority = self.winapi_priority;
        let boost_enabled = self.boost_enabled;
        let ideal_processor = self.ideal_processor;

        self.build_std().spawn(move || {
            let mut result = match (thread_priority, winapi_priority) {
                (Some(priority), None) => set_thread_priority(thread_native_id(), priority),
                (_, Some(priority)) => set_winapi_thread_priority(thread_native_id(), priority),
                _ => Ok(()),
            };
            if result.is_ok() && boost_enabled {
                result = set_current_thread_priority_boost(boost_enabled);
            }
            if result.is_ok() {
                if let Some(ideal_processor) = ideal_processor {
                    result = set_current_thread_ideal_processor(ideal_processor).map(|_| ());
                }
            }
            f(result)
        })
    }

    fn build_std(&mut self) -> std::thread::Builder {
        let mut builder = std::thread::Builder::new();

        if let Some(name) = &self.name {
            builder = builder.name(name.to_owned());
        }

        if let Some(stack_size) = self.stack_size {
            builder = builder.stack_size(stack_size);
        }

        builder
    }

    /// Spawns a new thread by taking ownership of the `Builder`, and returns an
    /// [`std::io::Result`] to its [`std::thread::JoinHandle`].
    ///
    /// See [`std::thread::Builder::spawn`]
    pub fn spawn_careless<F, T>(self, f: F) -> std::io::Result<std::thread::JoinHandle<T>>
    where
        F: FnOnce() -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.spawn(|priority_set_result| {
            if let Err(e) = priority_set_result {
                log::warn!(
                    "Couldn't set the priority for the thread with Rust Thread ID {:?} named {:?}: {:?}",
                    std::thread::current().id(),
                    std::thread::current().name(),
                    e,
                );
            }

            f()
        })
    }
}

/// Adds thread building functions using the priority.
pub trait ThreadBuilderExt {
    /// Spawn a thread with set priority. The passed functor `f` is executed in the spawned thread and
    /// receives as the only argument the result of setting the thread priority.
    /// See [`std::thread::Builder::spawn`] and [`ThreadPriority::set_for_current`] for more info.
    ///
    /// # Example
    ///
    /// ```rust
    /// use thread_priority::*;
    /// use thread_priority::ThreadBuilderExt;
    ///
    /// let thread = std::thread::Builder::new()
    ///     .name("MyNewThread".to_owned())
    ///     .spawn_with_priority(ThreadPriority::Max, |result| {
    ///         // This is printed out from within the spawned thread.
    ///         println!("Set priority result: {:?}", result);
    ///         assert!(result.is_ok());
    /// }).unwrap();
    /// thread.join();
    /// ```
    fn spawn_with_priority<F, T>(
        self,
        priority: ThreadPriority,
        f: F,
    ) -> std::io::Result<std::thread::JoinHandle<T>>
    where
        F: FnOnce(Result<(), Error>) -> T,
        F: Send + 'static,
        T: Send + 'static;
}

impl ThreadBuilderExt for std::thread::Builder {
    fn spawn_with_priority<F, T>(
        self,
        priority: ThreadPriority,
        f: F,
    ) -> std::io::Result<std::thread::JoinHandle<T>>
    where
        F: FnOnce(Result<(), Error>) -> T,
        F: Send + 'static,
        T: Send + 'static,
    {
        self.spawn(move || f(priority.set_for_current()))
    }
}

/// Spawns a thread with the specified priority.
///
/// See [`ThreadBuilderExt::spawn_with_priority`].
///
/// ```rust
/// use thread_priority::*;
///
/// let thread = spawn(ThreadPriority::Max, |result| {
///     // This is printed out from within the spawned thread.
///     println!("Set priority result: {:?}", result);
///     assert!(result.is_ok());
/// });
/// thread.join();
/// ```
pub fn spawn<F, T>(priority: ThreadPriority, f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce(Result<(), Error>) -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || f(priority.set_for_current()))
}

/// Spawns a thread with the specified priority.
/// This is different from [`spawn`] in a way that the passed function doesn't
/// need to accept the [`ThreadPriority::set_for_current`] result.
/// In case of an error, the error is logged using the logging facilities.
///
/// See [`spawn`].
///
/// ```rust
/// use thread_priority::*;
///
/// let thread = spawn_careless(ThreadPriority::Max, || {
///     // This is printed out from within the spawned thread.
///     println!("We don't care about the priority result.");
/// });
/// thread.join();
/// ```
pub fn spawn_careless<F, T>(priority: ThreadPriority, f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(e) = priority.set_for_current() {
            log::warn!(
                "Couldn't set the priority for the thread with Rust Thread ID {:?} named {:?}: {:?}",
                std::thread::current().id(),
                std::thread::current().name(),
                e,
            );
        }

        f()
    })
}
