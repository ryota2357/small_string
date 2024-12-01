use small_string::SmallString;

const INLINE_LIMIT: usize = size_of::<SmallString>();

#[test]
fn new_empty() {
    assert_eq!(SmallString::new(), "");

    let s = SmallString::new();
    assert_eq!(s.as_str(), "");
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert!(!s.is_heap_allocated());
    assert_eq!(s.capacity(), INLINE_LIMIT);
}

#[test]
fn new_from_char() {
    assert_eq!(SmallString::from('a'), "a");
    assert_eq!(SmallString::from('👍'), "👍");
    assert_eq!(SmallString::from(''), "");
}

#[test]
fn around_inline_limit() {
    let s = &String::from("0123456789abcdefg");

    let inline = SmallString::from(&s[..INLINE_LIMIT - 1]);
    assert_eq!(inline, s[..INLINE_LIMIT - 1]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let inline = SmallString::from(&s[..INLINE_LIMIT]);
    assert_eq!(inline, s[..INLINE_LIMIT]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let heap = SmallString::from(&s[..INLINE_LIMIT + 1]);
    assert_eq!(heap, s[..INLINE_LIMIT + 1]);
    assert!(heap.is_heap_allocated());
    assert_eq!(heap.capacity(), INLINE_LIMIT + 1);
}

#[test]
fn around_inline_limit_static() {
    let s: &'static str = "0123456789abcdefg";

    let inline = SmallString::from_static_str(&s[..INLINE_LIMIT - 1]);
    assert_eq!(inline, s[..INLINE_LIMIT - 1]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let inline = SmallString::from_static_str(&s[..INLINE_LIMIT]);
    assert_eq!(inline, s[..INLINE_LIMIT]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let static_ = SmallString::from_static_str(&s[..INLINE_LIMIT + 1]);
    assert_eq!(static_, s[..INLINE_LIMIT + 1]);
    assert!(!static_.is_heap_allocated());
    assert_eq!(static_.capacity(), INLINE_LIMIT + 1);
}
