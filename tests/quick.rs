use proptest::property_test;
use small_string::SmallString;

#[property_test]
fn create_from_str(s: String) {
    let s = s.as_str();
    let string = SmallString::from(s);
    assert_eq!(string.as_str(), s);
    assert_eq!(string.len(), s.len());
}
