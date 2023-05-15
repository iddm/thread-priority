#![cfg(not(windows))]

use rstest::rstest;
use std::convert::TryInto;
use thread_priority::*;

#[cfg(target_os = "linux")]
#[test]
fn get_and_set_priority_with_normal_and_crossplatform() {
    let nice = unsafe { libc::getpriority(0, 0) };
    assert_eq!(nice, 0);
    crate::set_current_thread_priority(ThreadPriority::Crossplatform(30u8.try_into().unwrap()))
        .unwrap();
    let nice = unsafe { libc::getpriority(0, 0) };
    assert!(nice > 0);
    // Note that increasing priority requires extra permissions (e.g. sudo)
    crate::set_current_thread_priority(ThreadPriority::Crossplatform(70u8.try_into().unwrap()))
        .unwrap();
    let nice = unsafe { libc::getpriority(0, 0) };
    assert!(nice < 0);
}

#[cfg(target_os = "linux")]
#[rstest]
fn get_and_set_priority_with_normal_policies(
    #[values(
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch)
    )]
    policy: ThreadSchedulePolicy,
    #[values(ThreadPriority::Min, ThreadPriority::Max, ThreadPriority::Crossplatform(23u8.try_into().unwrap()))]
    priority: ThreadPriority,
) {
    let ret = set_thread_priority_and_policy(thread_native_id(), priority, policy);
    if policy == ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle)
        && priority == ThreadPriority::Crossplatform(23u8.try_into().unwrap())
    {
        assert_eq!(ret, Err(Error::PriorityNotInRange(0..=0)));
    } else {
        assert!(ret.is_ok());
    }
}

// In macOS it is allowed to specify number as a SCHED_OTHER policy priority.
#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
#[rstest]
fn get_and_set_priority_with_normal_policies(
    #[values(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other))]
    policy: ThreadSchedulePolicy,
    #[values(ThreadPriority::Min, ThreadPriority::Max, ThreadPriority::Crossplatform(23u8.try_into().unwrap()))]
    priority: ThreadPriority,
) {
    assert!(set_thread_priority_and_policy(thread_native_id(), priority, policy).is_ok());
}

#[rstest]
#[cfg(target_os = "linux")]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle), 0..=0)]
#[cfg(target_os = "linux")]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch), -20..=19)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other), -20..=19)]
#[case(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo), 0..=99)]
#[case(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin), 0..=99)]
fn check_min_and_max_priority_values(
    #[case] policy: ThreadSchedulePolicy,
    #[case] posix_range: std::ops::RangeInclusive<i32>,
) {
    let max_value = ThreadPriority::max_value_for_policy(policy).unwrap();
    let min_value = ThreadPriority::min_value_for_policy(policy).unwrap();
    assert!(posix_range.contains(&max_value));
    assert!(posix_range.contains(&min_value));
}

#[rstest]
#[cfg(target_os = "linux")]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle))]
#[cfg(target_os = "linux")]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch))]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other))]
fn set_priority_with_normal_policy_but_with_invalid_value(#[case] policy: ThreadSchedulePolicy) {
    let thread_id = thread_native_id();
    #[cfg(target_os = "linux")]
    let expected = if policy == ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle) {
        // In Linux we should get an error whenever a non-zero value is passed as priority and a normal
        // scheduling policy is used.
        Err(Error::PriorityNotInRange(0..=0))
    } else {
        Ok(())
    };

    assert_eq!(
        set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
            policy,
        ),
        expected
    );
}

#[cfg(any(
    target_os = "macos",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "netbsd"
))]
#[test]
// In macOS the SCHED_OTHER policy allows having a non-zero priority value,
// but the crate doesn't use this opportunity for normal threads and uses niceness instead.
fn get_and_set_priority_with_normal_policy() {
    let thread_id = thread_native_id();
    let normal_policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other);

    assert_eq!(
        set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
            normal_policy,
        ),
        Ok(())
    );
    assert_eq!(thread_schedule_policy(), Ok(normal_policy));
}

#[rstest]
#[case::fifo(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo))]
#[case::roundrobin(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin))]
fn get_and_set_priority_with_realtime_policy_requires_capabilities(
    #[case] realtime_policy: ThreadSchedulePolicy,
) {
    let thread_id = thread_native_id();
    let max_value = ThreadPriority::max_value_for_policy(realtime_policy).unwrap();
    let min_value = ThreadPriority::min_value_for_policy(realtime_policy).unwrap();

    assert_eq!(
        set_thread_priority_and_policy(thread_id, ThreadPriority::Max, realtime_policy,),
        Ok(())
    );
    assert_eq!(thread_schedule_policy(), Ok(realtime_policy));
    assert_eq!(
        thread_schedule_policy_param(thread_native_id()),
        Ok((
            realtime_policy,
            ScheduleParams {
                sched_priority: max_value
            }
        ))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform((max_value as u8).try_into().unwrap()),
            id: thread_native_id()
        })
    );

    assert_eq!(
        set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
            realtime_policy,
        ),
        Ok(())
    );
    assert_eq!(thread_schedule_policy(), Ok(realtime_policy));
    assert_eq!(
        thread_schedule_policy_param(thread_native_id()),
        Ok((realtime_policy, ScheduleParams { sched_priority: 23 }))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
            id: thread_native_id()
        })
    );

    assert_eq!(
        set_thread_priority_and_policy(thread_id, ThreadPriority::Min, realtime_policy,),
        Ok(())
    );
    assert_eq!(thread_schedule_policy(), Ok(realtime_policy));
    assert_eq!(
        thread_schedule_policy_param(thread_native_id()),
        Ok((
            realtime_policy,
            ScheduleParams {
                sched_priority: min_value
            }
        ))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform((min_value as u8).try_into().unwrap()),
            id: thread_native_id()
        })
    );
}
