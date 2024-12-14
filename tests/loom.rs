// RUSTFLAGS="--cfg loom" cargo test --test loom --release
#![cfg(loom)]

use lean_string::LeanString;
use loom::thread;

#[test]
fn smoke() {
    loom::model(|| {
        let mut one = LeanString::from("12345678901234567890");
        let two = one.clone();

        thread::spawn(move || {
            let mut three = two.clone();
            three.push('a');

            assert_eq!(two, "12345678901234567890");
            assert_eq!(three, "12345678901234567890a");
        });

        one.push('a');
        assert_eq!(one, "12345678901234567890a");
    });
}
