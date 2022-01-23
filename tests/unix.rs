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
    assert!(set_thread_priority_and_policy(thread_native_id(), priority, policy,).is_err());
}

#[cfg(not(windows))]
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

#[cfg(not(windows))]
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
