use lean_string::LeanString;
use proptest::{prelude::*, property_test};

#[property_test]
#[cfg_attr(miri, ignore)]
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
#[cfg_attr(miri, ignore)]
fn create_from_u8_bytes(input: Vec<u8>) {
    let bytes = input.as_slice();

    let lean = LeanString::from_utf8(bytes);
    let string = String::from_utf8(bytes.to_vec());
    prop_assert_eq!(lean.is_err(), string.is_err());
    if let (Ok(lean), Ok(string)) = (lean, string) {
        prop_assert_eq!(&lean, &string);
    }

    let lean = LeanString::from_utf8_lossy(bytes);
    let string = String::from_utf8_lossy(bytes);
    prop_assert_eq!(&lean, &string);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn create_from_u16_bytes(input: Vec<u16>) {
    let bytes = input.as_slice();

    let lean = LeanString::from_utf16(bytes);
    let string = String::from_utf16(bytes);
    prop_assert_eq!(lean.is_err(), string.is_err());
    if let (Ok(lean), Ok(string)) = (lean, string) {
        prop_assert_eq!(&lean, &string);
    }

    let lean = LeanString::from_utf16_lossy(bytes);
    let string = String::from_utf16_lossy(bytes);
    prop_assert_eq!(&lean, &string);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn collect_from_chars(input: String) {
    let lean = input.chars().collect::<LeanString>();
    prop_assert_eq!(&lean, &input);
}

#[property_test]
#[cfg_attr(miri, ignore)]
fn collect_from_strings(input: Vec<String>) {
    let lean = input.clone().into_iter().collect::<LeanString>();
    let string = input.into_iter().collect::<String>();
    prop_assert_eq!(&lean, &string);
}
