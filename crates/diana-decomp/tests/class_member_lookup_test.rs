mod support;

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/classes/direct-initialization/classes.dex");

#[test]
fn member_indexes_follow_mutation_and_the_last_duplicate() {
    let engine = support::engine(DEX);
    let key = support::class(DEX, &engine, "Lfixture/Parent;");
    let facts = engine.class_facts(key).unwrap();
    let original = facts.node().unwrap();
    let mut class = original.clone();
    let mut method = class.find_method("<init>(I)V").unwrap().clone();
    let mut field = class.find_field("value:I").unwrap().clone();
    let method_id = method.id;
    let field_id = field.id;
    method.id += 1;
    field.id += 1;
    class.add_method(method);
    class.add_field(field);
    assert_eq!(class.find_method("<init>(I)V").unwrap().id, method_id + 1);
    assert_eq!(class.find_field("value:I").unwrap().id, field_id + 1);

    let snapshot = class.clone();
    class.find_method_mut("<init>(I)V").unwrap().info.name = "renamed".into();
    class.fields_mut()[1].info.name = "renamed".into();
    assert_eq!(class.find_method("<init>(I)V").unwrap().id, method_id);
    assert_eq!(class.find_field("value:I").unwrap().id, field_id);
    assert_eq!(class.find_method("renamed(I)V").unwrap().id, method_id + 1);
    assert_eq!(class.find_field("renamed:I").unwrap().id, field_id + 1);

    class.method_mut(1).unwrap().info.name = "<init>".into();
    assert_eq!(class.find_method("<init>(I)V").unwrap().id, method_id + 1);
    assert!(class.find_method("renamed(I)V").is_none());
    class.methods_mut().swap(0, 1);
    assert_eq!(class.find_method("<init>(I)V").unwrap().id, method_id);
    class.rebuild_caches();
    assert_eq!(class.find_method("<init>(I)V").unwrap().id, method_id);
    assert!(class.find_method("renamed(I)V").is_none());
    assert_eq!(
        snapshot.find_method("<init>(I)V").unwrap().id,
        method_id + 1
    );
    assert_eq!(snapshot.find_field("value:I").unwrap().id, field_id + 1);
}
