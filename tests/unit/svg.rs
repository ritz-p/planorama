use super::*;

#[test]
fn escapes_markup_and_filters_invalid_control_characters() {
    assert_eq!(
        escape("<script>&\"'\u{0}\u{1}\n\r\t"),
        "&lt;script&gt;&amp;&quot;&apos;\n\r\t"
    );
}
