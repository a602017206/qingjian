use notebook_core::Store;
use serde_json::json;

#[test]
fn invalid_templates_do_not_replace_existing_database() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::new(root.path()).unwrap();
    let id = store
        .dispatch("createWorkspace", json!({"name":"匿名模板"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let page=store.dispatch("save",json!({"workspaceId":id,"page":{"title":"原数据库","blocks":[],"database":{"columns":[{"id":"name","name":"名称","type":"text"}],"rows":[]}}})).unwrap();
    for templates in [
        json!({}),
        json!([{"id":"same","name":"甲","values":{},"blocks":[]},{"id":"same","name":"乙","values":{},"blocks":[]}]),
        json!([{"id":"template","name":"甲","values":{},"blocks":[{"id":"a","type":"paragraph","text":"甲","parentId":"a"}]}]),
        json!([{"id":"template","name":"甲","values":{"name":22},"blocks":[]}]),
    ] {
        let mut draft = page.clone();
        draft["database"]["templates"] = templates;
        assert!(store
            .dispatch("save", json!({"workspaceId":id,"page":draft}))
            .is_err());
        assert_eq!(
            store.dispatch("pages", json!({"workspaceId":id})).unwrap()[0],
            page
        );
    }
    let mut draft = page.clone();
    draft["database"]["defaultTemplateId"] = json!("missing");
    assert!(store
        .dispatch("save", json!({"workspaceId":id,"page":draft}))
        .is_err());
}
