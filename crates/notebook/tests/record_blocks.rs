use base64::{engine::general_purpose::STANDARD, Engine};
use notebook_core::Store;
use serde_json::{json, Value};

fn setup() -> (tempfile::TempDir, Store, String, Value) {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::new(root.path()).unwrap();
    let workspace = store
        .dispatch("createWorkspace", json!({"name":"记录正文匿名验证"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let page = store.dispatch("save", json!({"workspaceId":workspace,"page":{"title":"数据库","blocks":[],"database":{"columns":[{"id":"name","name":"名称","type":"text"}],"rows":[{"id":"row","values":{"name":"旧记录"},"body":"  原文\n\n末尾\n"}]}}})).unwrap();
    (root, store, workspace, page)
}

#[test]
fn invalid_record_blocks_are_rejected_without_overwriting_original() {
    let (_root, mut store, workspace, page) = setup();
    for blocks in [
        json!([{"id":"same","type":"paragraph","text":"甲"},{"id":"same","type":"paragraph","text":"乙"}]),
        json!([{"id":"cycle","parentId":"cycle","type":"paragraph","text":"原文"}]),
        json!({"text":"不是内容块数组"}),
    ] {
        let mut draft = page.clone();
        draft["database"]["rows"][0]["blocks"] = blocks;
        assert!(
            store
                .dispatch("save", json!({"workspaceId":workspace,"page":draft}))
                .is_err(),
            "无效记录正文必须拒绝保存"
        );
        assert_eq!(
            store
                .dispatch("pages", json!({"workspaceId":workspace}))
                .unwrap()[0],
            page
        );
    }
}

#[test]
fn record_block_changes_update_row_metadata_and_keep_legacy_text() {
    let (_root, mut store, workspace, mut page) = setup();
    page["database"]["rows"][0]["blocks"] =
        json!([{"id":"body","type":"paragraph","text":"  原文\n\n末尾\n"}]);
    let before = store
        .dispatch("save", json!({"workspaceId":workspace,"page":page}))
        .unwrap();
    let mut draft = before.clone();
    draft["database"]["rows"][0]["blocks"][0]["backgroundColor"] = json!("blue");
    std::thread::sleep(std::time::Duration::from_millis(2));
    let after = store
        .dispatch("save", json!({"workspaceId":workspace,"page":draft}))
        .unwrap();
    assert_ne!(
        before["database"]["rows"][0]["updatedAt"], after["database"]["rows"][0]["updatedAt"],
        "仅修改块样式也应更新记录编辑时间"
    );
    assert_eq!(after["database"]["rows"][0]["body"], "  原文\n\n末尾\n");
    assert_eq!(
        after["database"]["rows"][0]["blocks"][0]["text"],
        "  原文\n\n末尾\n"
    );
}

#[test]
fn structured_backup_remaps_record_block_page_references_and_preserves_layout() {
    let (_root, mut store, workspace, mut page) = setup();
    let linked = store
        .dispatch(
            "save",
            json!({"workspaceId":workspace,"page":{"title":"引用目标","blocks":[]}}),
        )
        .unwrap();
    let asset = store.dispatch("attach",json!({"workspaceId":workspace,"name":"正文附件.txt","data":STANDARD.encode(b"record attachment")})).unwrap();
    let body = format!(
        "[目标](page:{})\n[附件](asset:{})",
        linked["id"].as_str().unwrap(),
        asset["name"].as_str().unwrap()
    );
    page["database"]["rows"][0]["body"] = json!(body);
    page["database"]["rows"][0]["blockBody"] = json!(body);
    page["database"]["rows"][0]["blocks"] = json!([{"id":"toggle","type":"toggle","text":"父块"},{"id":"nested","parentId":"toggle","type":"paragraph","text":body}]);
    page["database"]["templates"] = json!([{ "id":"template","name":"模板","values":{},"blocks":[{"id":"template-body","type":"paragraph","text":body}]}]);
    store
        .dispatch("save", json!({"workspaceId":workspace,"page":page}))
        .unwrap();
    let exported = store
        .dispatch("export", json!({"workspaceId":workspace}))
        .unwrap();
    let other = store
        .dispatch("createWorkspace", json!({"name":"恢复目标"}))
        .unwrap()["id"]
        .clone();
    store
        .dispatch(
            "import",
            json!({"workspaceId":other,"name":"backup.zip","data":exported["data"]}),
        )
        .unwrap();
    let pages = store
        .dispatch("pages", json!({"workspaceId":other}))
        .unwrap();
    let restored = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "数据库")
        .unwrap();
    let target = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "引用目标")
        .unwrap();
    let row = &restored["database"]["rows"][0];
    assert_ne!(target["id"], linked["id"]);
    for value in [
        &row["body"],
        &row["blockBody"],
        &row["blocks"][1]["text"],
        &restored["database"]["templates"][0]["blocks"][0]["text"],
    ] {
        assert!(
            value
                .as_str()
                .unwrap()
                .contains(&format!("page:{}", target["id"].as_str().unwrap())),
            "记录正文引用必须映射到恢复后的页面"
        );
        assert!(!value
            .as_str()
            .unwrap()
            .contains(&format!("page:{}", linked["id"].as_str().unwrap())));
    }
    assert_eq!(row["blocks"][1]["parentId"], "toggle");
    let name = row["blocks"][1]["text"]
        .as_str()
        .unwrap()
        .split("asset:")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap();
    assert_eq!(
        store
            .dispatch("asset", json!({"workspaceId":other,"name":name}))
            .unwrap()["data"],
        STANDARD.encode(b"record attachment")
    );
}

#[test]
fn stale_record_block_save_does_not_replace_newer_content() {
    let (_root, mut store, workspace, page) = setup();
    let mut draft = page.clone();
    draft["database"]["rows"][0]["blocks"] = json!([{"id":"b","type":"paragraph","text":"新正文"}]);
    let saved = store
        .dispatch("save", json!({"workspaceId":workspace,"page":draft}))
        .unwrap();
    assert!(store
        .dispatch("save", json!({"workspaceId":workspace,"page":page}))
        .is_err());
    assert_eq!(
        store
            .dispatch("pages", json!({"workspaceId":workspace}))
            .unwrap()[0],
        saved
    );
}
