use super::*;

#[test]
fn long_dex_names_and_hints_retain_their_distinct_printing_rules_after_renaming() {
    let name = "same_long_spelling_in_a_class_and_a_method";
    let alias = "second_long_spelling_after_a_collision";
    let dex = JavaIdentifier::from_dex(name);
    let hinted = JavaIdentifier::from_hint(name);
    for original in [dex.clone(), JavaIdentifier::from_dex_alias(name, name)] {
        let changed = original.with_source_name(alias.to_owned());
        assert_eq!(original, hinted);
        assert_eq!(changed.as_str(), alias);
        assert_eq!(changed.original_name(), name);
        assert_eq!(with_preserved_dex_names(true, || changed.to_string()), name);
        assert_eq!(
            with_preserved_dex_names(false, || changed.to_string()),
            alias
        );
        assert_eq!(
            format!("{original:?}"),
            format!("JavaIdentifier {{ source_name: {name:?}, dex_name: Some({name:?}) }}")
        );
    }
    let changed = hinted.with_source_name(alias.to_owned());
    assert_eq!(changed.original_name(), alias);
    assert_eq!(
        with_preserved_dex_names(true, || changed.to_string()),
        alias
    );
    let sanitized = JavaIdentifier::from_dex("a-b");
    assert_eq!(sanitized.as_str(), "ab");
    assert_eq!(
        sanitized.with_source_name("ab3".into()).original_name(),
        "a-b"
    );
}
