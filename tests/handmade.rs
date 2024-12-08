use lean_string::LeanString;

const INLINE_LIMIT: usize = size_of::<LeanString>();

#[test]
fn new_empty() {
    assert_eq!(LeanString::new(), "");

    let s = LeanString::new();
    assert_eq!(s.as_str(), "");
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert!(!s.is_heap_allocated());
    assert_eq!(s.capacity(), INLINE_LIMIT);
}

#[test]
fn new_from_char() {
    assert_eq!(LeanString::from('a'), "a");
    assert_eq!(LeanString::from('👍'), "👍");
    assert_eq!(LeanString::from(''), "");
}

#[test]
fn from_around_inline_limit() {
    let s = &String::from("0123456789abcdefg");

    let inline = LeanString::from(&s[..INLINE_LIMIT - 1]);
    assert_eq!(inline, s[..INLINE_LIMIT - 1]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let inline = LeanString::from(&s[..INLINE_LIMIT]);
    assert_eq!(inline, s[..INLINE_LIMIT]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let heap = LeanString::from(&s[..INLINE_LIMIT + 1]);
    assert_eq!(heap, s[..INLINE_LIMIT + 1]);
    assert!(heap.is_heap_allocated());
    assert_eq!(heap.capacity(), INLINE_LIMIT + 1);
}

#[test]
fn from_around_inline_limit_static() {
    let s: &'static str = "0123456789abcdefg";

    let inline = LeanString::from_static_str(&s[..INLINE_LIMIT - 1]);
    assert_eq!(inline, s[..INLINE_LIMIT - 1]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let inline = LeanString::from_static_str(&s[..INLINE_LIMIT]);
    assert_eq!(inline, s[..INLINE_LIMIT]);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let static_ = LeanString::from_static_str(&s[..INLINE_LIMIT + 1]);
    assert_eq!(static_, s[..INLINE_LIMIT + 1]);
    assert!(!static_.is_heap_allocated());
    assert_eq!(static_.capacity(), INLINE_LIMIT + 1);
}

#[test]
fn push_cow() {
    let mut s = LeanString::new();
    s.push('a');
    s.push('b');
    s.push_str("cdefgh");
    assert_eq!(s, "abcdefgh");
    assert_eq!(s.len(), 8);

    s.push_str("12345678");
    assert_eq!(s.len(), 16);
    assert_eq!(s, "abcdefgh12345678");

    // clone and push
    let mut s1 = s.clone();
    assert_eq!(s1, "abcdefgh12345678");
    s1.push('0');
    assert_eq!(s1, "abcdefgh123456780");
    assert_eq!(s1.len(), 17);

    // clone and push_str
    let mut s2 = s.clone();
    s2.push_str("90");
    assert_eq!(s2, "abcdefgh1234567890");
    assert_eq!(s2.len(), 18);

    // s is not changed
    assert_eq!(s.len(), 16);

    // s into heap
    s.push_str("90");
    assert!(s.is_heap_allocated());
    assert_eq!(s.len(), 18);

    // clone and push
    let mut s3 = s.clone();
    s3.push('');
    assert_eq!(s3, "abcdefgh1234567890");
    assert_eq!(s3.len(), 21);

    // clone and push_str
    let mut s4 = s.clone();
    s4.push_str("👍👍");
    assert_eq!(s4.len(), 26);
    assert_eq!(s4, "abcdefgh1234567890👍👍");
}

#[test]
fn push_from_static() {
    let mut inline = LeanString::from_static_str("abcdefgh");
    assert_eq!(inline, "abcdefgh");
    assert_eq!(inline.len(), 8);
    assert!(!inline.is_heap_allocated());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    inline.push_str("12345678");
    assert_eq!(inline, "abcdefgh12345678");
    assert_eq!(inline.len(), 16);
    if cfg!(target_pointer_width = "64") {
        assert!(!inline.is_heap_allocated());
        assert_eq!(inline.capacity(), 16);
    } else {
        assert!(inline.capacity() >= 16);
    }

    inline.push_str("90");
    assert_eq!(inline, "abcdefgh1234567890");
    assert_eq!(inline.len(), 18);
    assert!(inline.is_heap_allocated());

    let mut static_ = LeanString::from_static_str("abcdefghijklmnopqrstuvwxyz");
    assert_eq!(static_, "abcdefghijklmnopqrstuvwxyz");
    assert_eq!(static_.len(), 26);
    assert!(!static_.is_heap_allocated());

    static_.push('0');
    assert_eq!(static_, "abcdefghijklmnopqrstuvwxyz0");
    assert_eq!(static_.len(), 27);
    assert!(static_.is_heap_allocated());
}

#[test]
fn pop_keep_capacity() {
    let mut inline = LeanString::from("Hello World!");
    assert_eq!(inline.pop(), Some('!'));
    assert_eq!(inline, "Hello World");
    assert_eq!(inline.len(), 11);

    for _ in 0..10 {
        inline.pop();
    }
    assert_eq!(inline, "H");
    assert_eq!(inline.pop(), Some('H'));
    assert_eq!(inline, "");
    assert!(inline.is_empty());
    assert_eq!(inline.capacity(), INLINE_LIMIT);

    let mut heap = LeanString::from("abcdefghijklmnopqrstuvwxyz");
    assert_eq!(heap.pop(), Some('z'));
    assert_eq!(heap, "abcdefghijklmnopqrstuvwxy");
    assert_eq!(heap.len(), 25);

    for _ in 0..24 {
        heap.pop();
    }
    assert_eq!(heap, "a");
    assert_eq!(heap.pop(), Some('a'));
    assert_eq!(heap, "");
    assert!(heap.is_empty());
    assert_eq!(heap.capacity(), 26);
}

#[test]
fn pop_cow() {
    let mut s = LeanString::from("abcdefgh");
    assert_eq!(s.pop(), Some('h'));
    assert_eq!(s.len(), 7);

    let mut s1 = s.clone();
    assert_eq!(s1.pop(), Some('g'));
    assert_eq!(s1, "abcdef");
    assert_eq!(s1.len(), 6);

    // s is not changed
    assert_eq!(s, "abcdefg");

    // s into heap
    s.push_str("hijklmnopqrstuvwxyz");

    let mut s2 = s.clone();
    assert_eq!(s2.pop(), Some('z'));
    assert_eq!(s2.len(), 25);

    // s is not changed
    assert_eq!(s, "abcdefghijklmnopqrstuvwxyz");
}
