use thread_priority;

fn main() {
    let current = thread_priority::get_current_thread_priority().unwrap();
    println!("Current thread priority is: {current:?}");
    let set_result = thread_priority::set_current_thread_priority(current);
    println!("Setting this priority again: {set_result:?}");
    assert_eq!(set_result, Ok(()));
}
