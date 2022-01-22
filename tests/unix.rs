#![cfg(not(windows))]

use rstest::rstest;
use thread_priority::*;

#[test]
fn get_and_set_priority_with_normal_policy() {
    use std::convert::TryInto;

    let thread_id = thread_native_id();
    #[cfg(not(target_os = "macos"))]
    let normal_policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal);
    #[cfg(target_os = "macos")]
    let normal_policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other);

    assert!(set_thread_priority_and_policy(thread_id, ThreadPriority::Min, normal_policy,).is_ok());
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

    assert!(set_thread_priority_and_policy(thread_id, ThreadPriority::Max, normal_policy,).is_ok());
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
#[test]
#[should_panic]
fn get_and_set_priority_with_normal_policy_with_invalid_value() {
    use std::convert::TryInto;

    let thread_id = thread_native_id();
    #[cfg(not(target_os = "macos"))]
    let normal_policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Normal);
    #[cfg(target_os = "macos")]
    let normal_policy = ThreadSchedulePolicy::Normal(NormalThreadSchedulePolicy::Other);

    assert!(set_thread_priority_and_policy(
        thread_id,
        ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
        normal_policy,
    )
    .is_ok());
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

    assert!(set_thread_priority_and_policy(
        thread_id,
        ThreadPriority::Crossplatform(23u8.try_into().unwrap()),
        realtime_policy,
    )
    .is_ok());
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
