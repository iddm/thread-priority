#![cfg(not(windows))]

use rstest::rstest;
use std::convert::TryInto;
use thread_priority::*;

#[cfg(linux)]
#[rstest]
fn get_and_set_priority_with_normal_policies(
    #[values(
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch)
    )]
    policy: ThreadSchedulePolicy,
    #[values(ThreadPriority::Min, ThreadPriority::Max)] correct_priority: ThreadPriority,
    #[values(ThreadPriority::Crossplatform(23u8.try_into().unwrap()))]
    incorrect_priority: ThreadPriority,
) {
    // In Linux it is only allowed to specify zero as a priority for normal scheduling policies.
    assert!(
        set_thread_priority_and_policy(thread_native_id(), incorrect_priority, policy,).is_err()
    );
    // For the case Min or Max is used, it is implicitly set to `0` so that there is no actual error.
    assert!(set_thread_priority_and_policy(thread_native_id(), correct_priority, policy,).is_ok());
}

#[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
#[rstest]
fn get_and_set_priority_with_normal_policies(
    #[values(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other))]
    policy: ThreadSchedulePolicy,
    #[values(ThreadPriority::Min, ThreadPriority::Max, ThreadPriority::Crossplatform(23u8.try_into().unwrap()))]
    priority: ThreadPriority,
) {
    // In macOS it is allowed to specify number as a SCHED_OTHER policy priority.
    assert!(set_thread_priority_and_policy(thread_native_id(), priority, policy,).is_ok());
}

#[rstest]
#[cfg(linux)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle), 0..=0)]
#[cfg(linux)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch), 0..=0)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other), 0..=0)]
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
#[cfg(linux)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle))]
#[cfg(linux)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch))]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other))]
fn set_priority_with_normal_policy_but_with_invalid_value(#[case] policy: ThreadSchedulePolicy) {
    use std::convert::TryInto;

    let thread_id = thread_native_id();

    assert_eq!(
        set_thread_priority_and_policy(
            thread_id,
            ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
            policy,
        ),
        // In Linux we should get an error whenever a non-zero value is passed as priority and a normal
        // scheduling policy is used.
        Err(Error::PriorityNotInRange(0..=0))
    );
}

#[cfg(any(target_os = "macos", target_os = "openbsd", target_os = "freebsd", target_os = "netbsd"))]
#[test]
// In macOS the SCHED_OTHER policy allows having a non-zero priority value.
fn get_and_set_priority_with_normal_policy() {
    use std::convert::TryInto;

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
    assert_eq!(
        thread_schedule_policy_param(thread_native_id()),
        Ok((normal_policy, ScheduleParams { sched_priority: 23 }))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
            id: thread_native_id()
        })
    );
}

#[rstest]
#[case::fifo(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo))]
#[case::roundrobin(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin))]
fn get_and_set_priority_with_realtime_policy_requires_capabilities(
    #[case] realtime_policy: ThreadSchedulePolicy,
) {
    use std::convert::TryInto;

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
