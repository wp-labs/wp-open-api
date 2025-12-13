use bytes::Bytes;
use std::sync::Arc;
use wp_parse_api::RawData;

#[test]
fn test_arc_bytes_creation() {
    let data = b"Hello, zero-copy world!";
    let arc_data = Arc::new(data.to_vec());
    let raw_data = RawData::from_arc_bytes(arc_data);

    assert!(raw_data.is_zero_copy());
    assert_eq!(raw_data.len(), data.len());
    assert!(!raw_data.is_empty());
}

#[test]
fn test_as_bytes_interface() {
    let original_data = b"Test data for as_bytes";
    let arc_data = Arc::new(original_data.to_vec());
    let raw_data = RawData::from_arc_bytes(arc_data);

    let bytes_slice = raw_data.as_bytes();
    assert_eq!(bytes_slice, original_data);
}

#[test]
fn test_to_bytes_conversion() {
    let original_data = b"Test data for to_bytes";
    let arc_data = Arc::new(original_data.to_vec());
    let raw_data = RawData::from_arc_bytes(arc_data);

    let bytes = raw_data.to_bytes();
    assert_eq!(bytes.as_ref(), original_data);
}

#[test]
fn test_zero_copy_detection() {
    let string_data = RawData::from_string("test string");
    let bytes_data = RawData::Bytes(Bytes::from("test bytes"));
    let arc_data = RawData::from_arc_bytes(Arc::new(b"test arc".to_vec()));

    assert!(!string_data.is_zero_copy());
    assert!(!bytes_data.is_zero_copy());
    assert!(arc_data.is_zero_copy());
}

#[test]
fn test_display_implementation() {
    let utf8_data = "Hello, 世界!";
    let arc_data = RawData::from_arc_bytes(Arc::new(utf8_data.as_bytes().to_vec()));

    let display_str = format!("{}", arc_data);
    assert_eq!(display_str, utf8_data);
}

#[test]
fn test_display_with_invalid_utf8() {
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    let arc_data = RawData::from_arc_bytes(Arc::new(invalid_utf8));

    let display_str = format!("{}", arc_data);
    // Should contain replacement characters for invalid UTF-8
    assert!(display_str.contains('�'));
}

#[test]
fn test_empty_arc_bytes() {
    let empty_arc = Arc::new(Vec::new());
    let raw_data = RawData::from_arc_bytes(empty_arc);

    assert!(raw_data.is_empty());
    assert_eq!(raw_data.len(), 0);
    assert!(raw_data.is_zero_copy());
}

#[test]
fn test_arc_sharing() {
    let original_data = b"Shared data test";
    let arc_data = Arc::new(original_data.to_vec());
    let raw_data1 = RawData::from_arc_bytes(Arc::clone(&arc_data));
    let raw_data2 = RawData::from_arc_bytes(Arc::clone(&arc_data));

    // Both should point to the same Arc
    assert!(raw_data1.is_zero_copy());
    assert!(raw_data2.is_zero_copy());
    assert_eq!(raw_data1.as_bytes(), raw_data2.as_bytes());
}
