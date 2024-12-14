// RUSTFLAGS="--cfg loom" cargo test --test loom --release
#![cfg(loom)]

use lean_string::LeanString;
use loom::{thread, thread::JoinHandle};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn model() -> JoinHandle<()> {
    let mut one = LeanString::from("12345678901234567890");
    let two = one.clone();

    let th = thread::spawn(move || {
        let mut three = two.clone();
        three.push('a');

        assert_eq!(two, "12345678901234567890");
        assert_eq!(three, "12345678901234567890a");
    });

    one.push('a');
    assert_eq!(one, "12345678901234567890a");

    th
}

#[test]
fn run() {
    loom::model(|| {
        let _profiler = dhat::Profiler::builder().testing().build();
        model().join().unwrap();
        let stats = dhat::HeapStats::get();
        // https://github.com/tokio-rs/loom/issues/369
        dhat::assert_eq!(stats.curr_blocks, 1);
    })
}
