use notebook_core::Store;
use serde_json::{json, Value};

fn diagram() -> Value {
    json!({"schemaVersion":1,"mode":"mindmap","nodes":[{"id":"root","text":"根","pageId":"","parentId":"","order":0,"collapsed":false},{"id":"child","text":"子","pageId":"","parentId":"root","order":0,"collapsed":false}],"edges":[]})
}
fn setup() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::new(dir.path()).unwrap();
    let w = store
        .dispatch("createWorkspace", json!({"name":"图测试"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (dir, store, w)
}
#[test]
fn invalid_tree_never_overwrites_saved_page() {
    let (_dir, mut s, w) = setup();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"图","blocks":[],"diagram":diagram()}}),
        )
        .unwrap();
    for nodes in [
        json!([]),
        json!([{"id":"root","text":"根","parentId":"root","order":0}]),
        json!([{"id":"root","text":"根","parentId":"","order":0},{"id":"child","text":"子","parentId":"missing","order":0}]),
        json!([{"id":"root","text":"根","parentId":"","order":0},{"id":"root","text":"子","parentId":"root","order":0}]),
    ] {
        let mut draft = page.clone();
        draft["diagram"]["nodes"] = nodes;
        assert!(s
            .dispatch("save", json!({"workspaceId":w,"page":draft}))
            .is_err());
        assert_eq!(
            s.dispatch("pages", json!({"workspaceId":w})).unwrap()[0],
            page
        );
    }
}
#[test]
fn backup_remaps_diagram_and_preview_references() {
    let (_dir, mut s, w) = setup();
    let target = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"目标","blocks":[]}}),
        )
        .unwrap();
    let mut d = diagram();
    d["nodes"][1]["pageId"] = target["id"].clone();
    let graph = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"图","blocks":[],"diagram":d}}),
        )
        .unwrap();
    let text = format!("[图](page:{})", graph["id"].as_str().unwrap());
    s.dispatch("save",json!({"workspaceId":w,"page":{"title":"正文","blocks":[{"id":"b","type":"paragraph","text":text,"diagramPreview":true}]}})).unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let dest = s
        .dispatch("createWorkspace", json!({"name":"恢复"}))
        .unwrap()["id"]
        .clone();
    s.dispatch(
        "import",
        json!({"workspaceId":dest,"name":"backup.zip","data":backup["data"]}),
    )
    .unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":dest})).unwrap();
    let find = |title| {
        pages
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["title"] == title)
            .unwrap()
    };
    let new_graph = find("图");
    let new_target = find("目标");
    assert_ne!(new_target["id"], target["id"]);
    assert_eq!(new_graph["diagram"]["nodes"][1]["pageId"], new_target["id"]);
    assert!(find("正文")["blocks"][0]["text"]
        .as_str()
        .unwrap()
        .contains(&format!("page:{}", new_graph["id"].as_str().unwrap())));
}
#[test]
fn newer_diagram_can_be_read_but_not_overwritten() {
    let (dir, mut s, w) = setup();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"未来图","blocks":[]}}),
        )
        .unwrap();
    let path = dir
        .path()
        .join("workspaces")
        .join(&w)
        .join("pages")
        .join(format!("{}.json", page["id"].as_str().unwrap()));
    let mut future = page.clone();
    future["diagram"] = json!({"schemaVersion":3,"opaque":{"must":"preserve"}});
    std::fs::write(&path, serde_json::to_vec(&future).unwrap()).unwrap();
    let loaded = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(loaded[0]["diagram"], future["diagram"]);
    let mut draft = loaded[0].clone();
    draft.as_object_mut().unwrap().remove("diagram");
    assert!(s
        .dispatch("save", json!({"workspaceId":w,"page":draft}))
        .is_err());
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap(),
        future
    );
}
#[test]
fn divergent_diagram_sync_keeps_both_versions() {
    let dir = tempfile::tempdir().unwrap();
    let remote = dir.path().join("remote.git");
    git2::Repository::init_bare(&remote).unwrap();
    let mut s = Store::new(&dir.path().join("data")).unwrap();
    let a = s
        .dispatch("createWorkspace", json!({"name":"甲","remote":remote}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":a,"page":{"title":"图","blocks":[],"diagram":diagram()}}),
        )
        .unwrap();
    s.dispatch("sync", json!({"workspaceId":a})).unwrap();
    let b = s
        .dispatch(
            "createWorkspace",
            json!({"name":"乙","remote":remote,"mode":"clone"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut pa = page.clone();
    pa["diagram"]["nodes"][1]["text"] = json!("甲修改");
    s.dispatch("save", json!({"workspaceId":a,"page":pa}))
        .unwrap();
    let mut pb = s.dispatch("pages", json!({"workspaceId":b})).unwrap()[0].clone();
    pb["diagram"]["nodes"][1]["text"] = json!("乙修改");
    s.dispatch("save", json!({"workspaceId":b,"page":pb}))
        .unwrap();
    s.dispatch("sync", json!({"workspaceId":a})).unwrap();
    let conflict = s.dispatch("sync", json!({"workspaceId":b})).unwrap();
    assert_eq!(conflict["status"], "conflict");
    assert_eq!(conflict["conflicts"][0]["local"]["diagram"]["nodes"], 2);
    assert!(conflict["conflicts"][0]["local"]["diagram"]["text"]
        .as_str()
        .unwrap()
        .contains("乙修改"));
    assert!(conflict["conflicts"][0]["remote"]["diagram"]["text"]
        .as_str()
        .unwrap()
        .contains("甲修改"));
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":b})).unwrap()[0]["diagram"]["nodes"][1]["text"],
        "乙修改"
    );
    let path = format!("pages/{}.json", page["id"].as_str().unwrap());
    s.dispatch(
        "resolveSync",
        json!({"workspaceId":b,"preview":conflict,"choices":{path:"remote"}}),
    )
    .unwrap();
    let backups = s.dispatch("syncBackups", json!({"workspaceId":b})).unwrap();
    assert!(!backups.as_array().unwrap().is_empty());
    let restored = s
        .dispatch(
            "restoreSyncBackup",
            json!({"workspaceId":b,"backupId":backups[0]["id"],"side":"local"}),
        )
        .unwrap();
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":restored["id"]}))
            .unwrap()[0]["diagram"]["nodes"][1]["text"],
        "乙修改"
    );
}
#[test]
fn backup_reports_missing_diagram_target() {
    let (_dir, mut s, w) = setup();
    let mut d = diagram();
    d["nodes"][1]["pageId"] = json!("missing-page");
    s.dispatch(
        "save",
        json!({"workspaceId":w,"page":{"title":"图","blocks":[],"diagram":d}}),
    )
    .unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let dest = s
        .dispatch("createWorkspace", json!({"name":"恢复"}))
        .unwrap()["id"]
        .clone();
    let result = s
        .dispatch(
            "import",
            json!({"workspaceId":dest,"name":"backup.zip","data":backup["data"]}),
        )
        .unwrap();
    assert!(result["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w.as_str().unwrap().contains("missing-page")));
    let pages = s.dispatch("pages", json!({"workspaceId":dest})).unwrap();
    assert_eq!(pages[0]["diagram"]["nodes"][1]["pageId"], "");
    assert_eq!(
        pages[0]["diagram"]["nodes"][1]["unresolvedPageId"],
        "missing-page"
    );
}
#[test]
fn backup_readable_markdown_includes_collapsed_diagram_nodes() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::io::Read;
    let (_dir, mut s, w) = setup();
    let mut d = diagram();
    d["nodes"][0]["collapsed"] = json!(true);
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"图","blocks":[],"diagram":d}}),
        )
        .unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let bytes = STANDARD.decode(backup["data"].as_str().unwrap()).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut markdown = String::new();
    zip.by_name(&format!("markdown/{}.md", page["id"].as_str().unwrap()))
        .unwrap()
        .read_to_string(&mut markdown)
        .unwrap();
    assert!(markdown.contains("- 根"));
    assert!(markdown.contains("  - 子"));
}
#[test]
fn stale_diagram_save_cannot_replace_newer_content() {
    let (_dir, mut s, w) = setup();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"图","blocks":[],"diagram":diagram()}}),
        )
        .unwrap();
    let mut newer = page.clone();
    newer["diagram"]["nodes"][1]["text"] = json!("新版本");
    let saved = s
        .dispatch("save", json!({"workspaceId":w,"page":newer}))
        .unwrap();
    let mut stale = page.clone();
    stale["diagram"]["nodes"][1]["text"] = json!("过期版本");
    assert!(s
        .dispatch("save", json!({"workspaceId":w,"page":stale}))
        .is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w})).unwrap()[0],
        saved
    );
}
#[test]
fn diagram_reference_fields_must_be_strings() {
    let (_dir, mut s, w) = setup();
    for key in ["pageId", "parentId"] {
        let mut d = diagram();
        d["nodes"][1][key] = json!(42);
        assert!(
            s.dispatch(
                "save",
                json!({"workspaceId":w,"page":{"title":"非法图","blocks":[],"diagram":d}})
            )
            .is_err(),
            "{key} 非字符串不可保存"
        );
    }
}
