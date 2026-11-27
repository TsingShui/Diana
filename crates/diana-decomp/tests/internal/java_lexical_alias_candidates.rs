use super::*;
use crate::language::java::{
    with_preserved_dex_names, JavaAssignOp, JavaFieldDeclaration, JavaMethodBody,
    JavaMethodDeclaration, JavaTypeDeclarationKind,
};

fn outer(original: &str) -> JavaType {
    source_type(&[JavaIdentifier::from_dex_alias(original, "Outer")])
}

fn method(
    kind: JavaMethodDeclarationKind,
    names: &[&str],
    spelling: &str,
) -> JavaMethodDeclaration {
    let writes = names
        .iter()
        .map(|name| JavaStmt::Assign {
            target: JavaExpr::Field {
                owner: Box::new(JavaExpr::This),
                name: JavaIdentifier::from_dex(name),
            },
            op: JavaAssignOp::Assign,
            value: JavaExpr::QualifiedThis(outer(spelling)),
        })
        .collect();
    JavaMethodDeclaration {
        annotations: vec![],
        modifiers: vec![],
        compiler_generated: false,
        kind,
        type_parameters: vec![],
        return_type: None,
        name: None,
        parameters: vec![],
        throws: vec![],
        body: Some(JavaMethodBody {
            root: JavaStmt::Block(writes),
        }),
    }
}

#[test]
fn alias_must_have_one_write_per_constructor_and_keep_the_first_outer_spelling() {
    let fields = ["kept", "missing", "repeated", "modified"]
        .into_iter()
        .map(|name| JavaFieldDeclaration {
            annotations: vec![],
            modifiers: vec![JavaModifier::Final],
            ty: JavaType::source_class("Outer"),
            name: JavaIdentifier::from_dex(name),
            initializer: None,
        })
        .collect();
    let mut declaration = JavaTypeDeclaration {
        annotations: vec![],
        modifiers: vec![],
        kind: JavaTypeDeclarationKind::Class,
        name: JavaIdentifier::from_dex("Inner"),
        type_parameters: vec![],
        extends: None,
        implements: vec![],
        enum_constants: vec![],
        fields,
        methods: vec![
            method(
                JavaMethodDeclarationKind::Method,
                &["modified"],
                "ExternalWrite",
            ),
            method(
                JavaMethodDeclarationKind::Constructor,
                &["kept", "repeated", "repeated", "modified", "missing"],
                "FirstOuter",
            ),
            method(
                JavaMethodDeclarationKind::Constructor,
                &["kept", "repeated", "modified"],
                "SecondOuter",
            ),
        ],
        nested: vec![],
    };
    let aliases = LexicalAliasAnalysis::analyze(&mut declaration, &JavaType::source_class("Inner"));
    assert_eq!(aliases.len(), 1);
    let (key, target) = aliases.first_key_value().unwrap();
    assert_eq!(key.name.as_str(), "kept");
    assert_eq!(
        with_preserved_dex_names(true, || target.to_string()),
        "FirstOuter"
    );
}
