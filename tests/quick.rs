use lean_string::LeanString;
use proptest::property_test;

#[property_test]
fn create_from_str(s: String) {
    let s = s.as_str();
    let string = LeanString::from(s);
    assert_eq!(string.as_str(), s);
    assert_eq!(string.len(), s.len());
}
