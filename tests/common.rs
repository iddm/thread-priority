use rstest::rstest;

#[rstest]
fn should_be_possible_to_reset_the_same_priority() {
    let current = thread_priority::get_current_thread_priority().unwrap();
    let set_result = thread_priority::set_current_thread_priority(current);
    assert_eq!(set_result, Ok(()));
}
