use lean_string::LeanString;
use proptest::{prelude::*, property_test};

#[property_test]
fn create_from_str(input: String) {
    let str = input.as_str();
    let lean = LeanString::from(str);
    prop_assert_eq!(&lean, str);
    prop_assert_eq!(lean.len(), str.len());

    if str.len() <= 2 * size_of::<usize>() {
        prop_assert!(!lean.is_heap_allocated());
    } else {
        prop_assert!(lean.is_heap_allocated());
    }
}

#[property_test]
fn collect_from_chars(input: String) {
    let lean = input.chars().collect::<LeanString>();
    prop_assert_eq!(&lean, &input);
}

#[property_test]
fn collect_from_strings(input: Vec<String>) {
    let lean = input.clone().into_iter().collect::<LeanString>();
    let string = input.into_iter().collect::<String>();
    prop_assert_eq!(&lean, &string);
}
