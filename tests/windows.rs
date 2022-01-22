use rstest::rstest;
use thread_priority::*;

// #[rstest]
// #[case::fifo(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::Fifo))]
// #[case::roundrobin(ThreadSchedulePolicy::Realtime(RealtimeThreadSchedulePolicy::RoundRobin))]
// fn get_and_set_priority_with_realtime_policy_requires_capabilities(
//     #[case] realtime_policy: ThreadSchedulePolicy,
// ) {
//     use std::convert::TryInto;

//     let thread_id = thread_native_id();

//     let set_result = set_winapi_thread_priority(thread_id, ThreadPriority::Max);
//     let get_result = thread_priority(thread_id, ThreadPriority::Max);
//     assert_eq!(
//         ,
//         Ok(())
//     );
//     assert_eq!(thread_schedule_policy(), Ok(realtime_policy));
//     assert_eq!(
//         thread_schedule_policy_param(thread_native_id()),
//         Ok((realtime_policy, ScheduleParams { sched_priority: 99 }))
//     );
//     assert_eq!(
//         Thread::current(),
//         Ok(Thread {
//             priority: ThreadPriority::Crossplatform(99u8.try_into().unwrap()),
//             id: thread_native_id()
//         })
//     );
// }
