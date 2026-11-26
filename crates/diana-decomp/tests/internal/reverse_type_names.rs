use super::{ArgType, JavaClassName, JavaClassType, JavaIdentifier, JavaTypeNameResolver};
use crate::language::java::with_preserved_dex_names;

fn resolver() -> JavaTypeNameResolver {
    JavaTypeNameResolver::for_test(None, Some("p"), Some(&ArgType::object("p/Outer")), [])
        .expect("type names")
}

#[test]
fn exact_spelling_prefers_first_import_then_body_then_header() {
    let mut names = resolver();
    names.simple_names.insert("y.B".into(), "T".into());
    names.simple_names.insert("x.A".into(), "T".into());
    for (binary, spelling) in [("p.A", "T"), ("p.B", "Nested")] {
        names
            .declarations
            .source_names
            .insert(binary.into(), JavaClassType::from_source(spelling));
    }
    names
        .declarations
        .header_names
        .insert("p.A".into(), JavaClassType::from_source("Nested"));
    let lookup = |spelling| names.reverse_names.binary_name(&names, spelling);
    assert_eq!(lookup("T"), Some("x.A"));
    assert_eq!(lookup("Nested"), Some("p.B"));
    assert!(names.reverse_names.fallback.get().is_none());
    assert_eq!(lookup("missing"), None);
}

#[test]
fn ambiguous_relative_defers_to_qualified_and_duplicate_binaries_stay_unique() {
    let mut names = resolver();
    for (binary, spelling) in [
        ("p.Outer$1", "Alias1"),
        ("p.Outer$Anonymous1", "Alias2"),
        ("p.Outer$Leaf", "Alias3"),
        ("Anonymous1", "Alias4"),
        ("Leaf", "Alias5"),
    ] {
        names.simple_names.insert(binary.into(), spelling.into());
    }
    names
        .declarations
        .header_names
        .insert("p.Outer$Leaf".into(), JavaClassType::from_source("Other"));
    let lookup = |spelling| names.reverse_names.binary_name(&names, spelling);
    // The two distinct nested binaries both have relative spelling Anonymous1.
    assert_eq!(lookup("Anonymous1"), Some("Anonymous1"));
    assert_eq!(lookup("p.Outer.Anonymous1"), Some("p.Outer$1"));
    assert_eq!(lookup("Leaf"), Some("p.Outer$Leaf"));
    assert_eq!(lookup("p.Outer.NoClass"), None);
}

#[test]
fn reused_resolver_matches_the_active_spelling_for_overlapping_dex_names() {
    let mut names = resolver();
    let declared = |dex, source: &str| {
        JavaClassType::raw(JavaClassName::simple(JavaIdentifier::from_dex_alias(
            dex,
            source.to_owned(),
        )))
    };
    names
        .declarations
        .source_names
        .insert("p.A".into(), declared("Raw", "Shown"));
    names
        .declarations
        .source_names
        .insert("p.B".into(), declared("Shown", "Other"));
    let lookup = |spelling| names.reverse_names.binary_name(&names, spelling);
    assert_eq!(lookup("Shown"), Some("p.A"));
    with_preserved_dex_names(true, || {
        assert_eq!(lookup("Raw"), Some("p.A"));
        assert_eq!(lookup("Shown"), Some("p.B"));
        assert_eq!(lookup("Other"), None);
    });
    assert_eq!(lookup("Other"), Some("p.B"));
    assert_eq!(lookup("Raw"), None);
}

#[test]
fn large_compilation_unit_resolves_all_imports_and_retains_unparseable_exact_binary() {
    let mut names = resolver();
    for i in 0..512 {
        names
            .simple_names
            .insert(format!("p.T{i}"), format!("T{i}"));
    }
    names.simple_names.insert("bad.".into(), "Invalid".into());
    names
        .declarations
        .source_names
        .insert("p.Valid".into(), JavaClassType::from_source("Invalid"));
    for i in (0..512).rev() {
        let ty = JavaClassType::from_source(&format!("T{i}"));
        assert_eq!(
            names.source_class_signature(&ty).unwrap().erased_name(),
            format!("p/T{i}")
        );
    }
    assert!(names
        .source_class_signature(&JavaClassType::from_source("Invalid"))
        .is_none());
    assert!(names.reverse_names.fallback.get().is_none());
}
