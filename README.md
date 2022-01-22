# thread-priority
[![CI](https://github.com/vityafx/thread-priority/actions/workflows/ci.yml/badge.svg)](https://github.com/vityafx/thread-priority/actions/workflows/ci.yml)
[![Crates](https://img.shields.io/crates/v/thread-priority.svg)](https://crates.io/crates/thread-priority)
[![Docs](https://docs.rs/thread-priority/badge.svg)](https://docs.rs/thread-priority)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)


A simple library to control thread schedule policies and thread priority.

If your operating system isn't yet supported, please, create an issue.

## Minimal Rust Compiler Version
Is `1.46`. If you need any help making it possible to compile with `1.36` please reach out.

## Supported platforms
- Linux
- DragonFly
- FreeBSD
- OpenBSD
- NetBSD
- macOS
- Windows

## Examples

### Minimal cross-platform examples
Setting current thread's priority to minimum:

```rust,no_run
use thread_priority::*;

fn main() {
    assert!(set_current_thread_priority(ThreadPriority::Min).is_ok());
}
```

The same as above but using a specific value:

```rust,no_run
use thread_priority::*;
use std::convert::TryInto;

fn main() {
    // The lower the number the lower the priority.
    assert!(set_current_thread_priority(ThreadPriority::Crossplatform(0.try_into().unwrap())).is_ok());
}
```

### Windows-specific examples
Set the thread priority to the lowest possible value:

```rust,no_run
use thread_priority::*;

fn main() {
    // The lower the number the lower the priority.
    assert!(set_current_thread_priority(ThreadPriority::Os(WinAPIThreadPriority::Lowest.into())).is_ok());
}
```

Set the ideal processor for the new thread:

```rust,no_run
use thread_priority::*;

fn main() {
    std::thread::spawn(|| {
        set_thread_ideal_processor(thread_native_id(), 0);
        println!("Hello world!");
    });
}
```


### Building a thread using the ThreadBuilderExt trait

```rust,no_run
use thread_priority::*;
use thread_priority::ThreadBuilderExt;

let thread = std::thread::Builder::new()
    .name("MyNewThread".to_owned())
    .spawn_with_priority(ThreadPriority::Max, |result| {
        // This is printed out from within the spawned thread.
        println!("Set priority result: {:?}", result);
        assert!(result.is_ok());
}).unwrap();
thread.join();
```

### Building a thread using the ThreadBuilder.

```rust,no_run
use thread_priority::*;

let thread = ThreadBuilder::default()
    .name("MyThread")
    .priority(ThreadPriority::Max)
    .spawn(|result| {
        // This is printed out from within the spawned thread.
        println!("Set priority result: {:?}", result);
        assert!(result.is_ok());
}).unwrap();
thread.join();

// Another example where we don't care about the priority having been set.
let thread = ThreadBuilder::default()
    .name("MyThread")
    .priority(ThreadPriority::Max)
    .spawn_careless(|| {
        // This is printed out from within the spawned thread.
        println!("We don't care about the priority result.");
}).unwrap();
thread.join();
```

### Using ThreadExt trait on the current thread

```rust,no_run
use thread_priority::*;

assert!(std::thread::current().get_priority().is_ok());
println!("This thread's native id is: {:?}", std::thread::current().get_native_id());
```

## License
This project is [licensed under the MIT license](https://github.com/vityafx/thread-priority/blob/master/LICENSE).
