#![cfg(windows)]
use rstest::rstest;
use thread_priority::*;
use std::convert::TryInto;

#[rstest]
#[case(ThreadPriority::Min, ThreadPriority::Os(WinAPIThreadPriority::Lowest.try_into().unwrap()))]
#[case(ThreadPriority::Crossplatform(23u8.try_into().unwrap()), ThreadPriority::Os(WinAPIThreadPriority::BelowNormal.try_into().unwrap()))]
#[case(ThreadPriority::Max, ThreadPriority::Os(WinAPIThreadPriority::Highest.try_into().unwrap()))]
fn get_and_set_priority_requires_capabilities(
    #[case] input_priority: ThreadPriority,
    #[case] expected_priority: ThreadPriority,
) {
    let thread_id = thread_native_id();

    let set_result = set_thread_priority(thread_id, input_priority);
    let get_result = get_thread_priority(thread_id);
    assert_eq!(
        set_result,
        Ok(())
    );
    assert_eq!(
        get_result,
        Ok(expected_priority),
    );
}
