// Temporary CI reporting demonstrations. Remove this file before merging.

#[test]
fn intentional_test_report_demo() {
    let actual = String::from("Pikachu");
    assert_eq!(
        actual, "Eevee",
        "Intentional failure to demonstrate the GitHub test report; remove before merging"
    );
}

#[test]
fn intentional_clippy_annotation_demo() {
    let visible = std::hint::black_box(true);
    // Deliberately use assert_eq! with a boolean to demonstrate an inline Clippy annotation.
    assert_eq!(visible, true);
}
