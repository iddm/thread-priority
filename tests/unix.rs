#![cfg(not(windows))]

use rstest::rstest;
use std::convert::TryInto;
use thread_priority::*;

#[rstest]
#[cfg(not(target_os = "macos"))]
fn get_and_set_priority_with_normal_policies(
    #[values(
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle),
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Batch)
    )]
    policy: ThreadSchedulePolicy,
    #[values(ThreadPriority::Min, ThreadPriority::Max, ThreadPriority::Crossplatform(23u8.try_into().unwrap()))]
    priority: ThreadPriority,
) {
    assert!(set_thread_priority_and_policy(thread_native_id(), priority, policy,).is_err());
}

#[rstest]
#[cfg(target_os = "macos")]
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
#[cfg(not(target_os = "macos"))]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal), 0..=0)]
#[case(ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Idle), 0..=0)]
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
#[cfg(target_os = "macos")]
fn check_min_and_max_priority_values(
    #[values(
        ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo),
        ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin)
    )]
    policy: ThreadSchedulePolicy,
) {
    let posix_range = 0..=99;

    let max_value = ThreadPriority::max_value_for_policy(policy).unwrap();
    let min_value = ThreadPriority::min_value_for_policy(policy).unwrap();
    assert!(posix_range.contains(&max_value));
    assert!(posix_range.contains(&min_value));
}

#[test]
#[should_panic]
fn get_and_set_priority_with_normal_policy_with_invalid_value() {
    use std::convert::TryInto;

    let thread_id = thread_native_id();
    #[cfg(not(target_os = "macos"))]
    let normal_policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal);
    #[cfg(target_os = "macos")]
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
        Ok((normal_policy, ScheduleParams { sched_priority: 0 }))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform(0u8.try_into().unwrap()),
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

    assert_eq!(
        set_thread_priority_and_policy(thread_id, ThreadPriority::Max, realtime_policy,),
        Ok(())
    );
    assert_eq!(thread_schedule_policy(), Ok(realtime_policy));
    assert_eq!(
        thread_schedule_policy_param(thread_native_id()),
        Ok((realtime_policy, ScheduleParams { sched_priority: 99 }))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform(99u8.try_into().unwrap()),
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
        Ok((realtime_policy, ScheduleParams { sched_priority: 1 }))
    );
    assert_eq!(
        Thread::current(),
        Ok(Thread {
            priority: ThreadPriority::Crossplatform(1u8.try_into().unwrap()),
            id: thread_native_id()
        })
    );
}
