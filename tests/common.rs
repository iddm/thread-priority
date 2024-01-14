use rstest::rstest;

#[rstest]
fn should_be_possible_to_reset_the_same_priority() -> Result<(), Box<dyn std::error::Error>> {
    let current = thread_priority::get_current_thread_priority()?;
    thread_priority::set_current_thread_priority(current)?;
    Ok(())
}

#[rstest]
fn should_be_possible_to_get_current_thread_native_id_via_threadext() {
    use thread_priority::ThreadExt;

    let current = std::thread::current();
    #[cfg(unix)]
    assert_eq!(
        current.get_native_id(),
        Ok(thread_priority::unix::thread_native_id())
    );
    #[cfg(windows)]
    assert_eq!(
        current.get_native_id(),
        Ok(thread_priority::windows::thread_native_id())
    );
}

#[rstest]
fn should_be_impossible_to_get_other_thread_native_id_via_threadext() {
    use thread_priority::ThreadExt;

    let current = std::thread::current();
    let another_thread = std::thread::spawn(move || {
        #[cfg(unix)]
        assert!(current.get_native_id().is_err());
        #[cfg(windows)]
        assert!(current.get_native_id().is_err());
    });
    another_thread.join().unwrap();
}
