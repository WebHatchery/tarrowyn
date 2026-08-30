use super::encode_query_value;

#[test]
fn query_encoding_preserves_search_words_and_escapes_form_symbols() {
    assert_eq!(
        encode_query_value("Storm Magic & rain"),
        "Storm%20Magic%20%26%20rain"
    );
}
