use notebook_core::Store;
use serde_json::{json, Value};
fn setup() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = s
        .dispatch("createWorkspace", json!({"name":"自由节点图测试"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    (dir, s, w)
}
fn diagram() -> Value {
    json!({"schemaVersion":2,"mode":"freeform","nodes":[{"id":"a","text":"甲","x":30,"y":50,"pageId":""},{"id":"b","text":"乙","x":320,"y":210,"pageId":""}],"edges":[{"id":"e1","sourceId":"a","targetId":"b","directed":true,"label":"依赖"},{"id":"e2","sourceId":"b","targetId":"a","directed":false,"label":"关联"}]})
}
#[test]
fn freeform_valid_roundtrip_and_invalid_rejected_without_overwrite() {
    let (_dir, mut s, w) = setup();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"自由图","blocks":[],"diagram":diagram()}}),
        )
        .unwrap();
    assert_eq!(page["diagram"], diagram());
    for patch in [
        json!({"nodes":[{"id":"a","text":"甲","x":1,"y":2},{"id":"a","text":"乙","x":3,"y":4}],"edges":[]}),
        json!({"nodes":[{"id":"a","text":"甲","x":"bad","y":2}],"edges":[]}),
        json!({"nodes":diagram()["nodes"],"edges":[{"id":"e","sourceId":"a","targetId":"missing","directed":true,"label":""}]}),
        json!({"nodes":diagram()["nodes"],"edges":[{"id":"e","sourceId":"a","targetId":"b","directed":"yes","label":""}]}),
        json!({"nodes":diagram()["nodes"],"edges":[{"id":"e","sourceId":"a","targetId":"b","directed":true,"label":""},{"id":"e","sourceId":"b","targetId":"a","directed":true,"label":""}]}),
    ] {
        let mut draft = page.clone();
        draft["diagram"]["nodes"] = patch["nodes"].clone();
        draft["diagram"]["edges"] = patch["edges"].clone();
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
fn backup_remaps_page_references_and_keeps_edges_positions_and_preview() {
    let (_dir, mut s, w) = setup();
    let target = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"目标","blocks":[]}}),
        )
        .unwrap();
    let mut d = diagram();
    d["nodes"][0]["pageId"] = target["id"].clone();
    let graph = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"自由图","blocks":[],"diagram":d}}),
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
    let find = |t| {
        pages
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["title"] == t)
            .unwrap()
    };
    let restored = find("自由图");
    assert_eq!(
        restored["diagram"]["nodes"][0]["pageId"],
        find("目标")["id"]
    );
    assert_eq!(restored["diagram"]["nodes"][1]["x"], 320);
    assert_eq!(restored["diagram"]["edges"], graph["diagram"]["edges"]);
    assert!(find("正文")["blocks"][0]["text"]
        .as_str()
        .unwrap()
        .contains(&format!("page:{}", restored["id"].as_str().unwrap())));
}
#[test]
fn readable_backup_keeps_node_labels_and_directed_edges() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::io::Read;
    let (_dir, mut s, w) = setup();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"自由图","blocks":[],"diagram":diagram()}}),
        )
        .unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let bytes = STANDARD.decode(backup["data"].as_str().unwrap()).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut text = String::new();
    zip.by_name(&format!("markdown/{}.md", page["id"].as_str().unwrap()))
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert!(text.contains("甲"));
    assert!(text.contains("依赖"));
    assert!(text.contains("→"));
    assert!(text.contains("—"));
}
#[test]
fn same_graph_conflict_keeps_both_node_positions_and_edges() {
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
            json!({"workspaceId":a,"page":{"title":"自由图","blocks":[],"diagram":diagram()}}),
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
    pa["diagram"]["nodes"][0]["x"] = json!(800);
    s.dispatch("save", json!({"workspaceId":a,"page":pa}))
        .unwrap();
    let mut pb = s.dispatch("pages", json!({"workspaceId":b})).unwrap()[0].clone();
    pb["diagram"]["edges"][0]["label"] = json!("本机标签");
    s.dispatch("save", json!({"workspaceId":b,"page":pb}))
        .unwrap();
    s.dispatch("sync", json!({"workspaceId":a})).unwrap();
    let conflict = s.dispatch("sync", json!({"workspaceId":b})).unwrap();
    assert_eq!(conflict["status"], "conflict");
    assert_eq!(
        conflict["conflicts"][0]["local"]["diagram"]["mode"],
        "freeform"
    );
    assert!(conflict["conflicts"][0]["local"]["diagram"]["text"]
        .as_str()
        .unwrap()
        .contains("本机标签"));
    let path = format!("pages/{}.json", page["id"].as_str().unwrap());
    s.dispatch(
        "resolveSync",
        json!({"workspaceId":b,"preview":conflict,"choices":{path:"remote"}}),
    )
    .unwrap();
    let backups = s.dispatch("syncBackups", json!({"workspaceId":b})).unwrap();
    let restored = s
        .dispatch(
            "restoreSyncBackup",
            json!({"workspaceId":b,"backupId":backups[0]["id"],"side":"local"}),
        )
        .unwrap();
    let local = s
        .dispatch("pages", json!({"workspaceId":restored["id"]}))
        .unwrap();
    assert_eq!(local[0]["diagram"]["edges"][0]["label"], "本机标签");
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":b})).unwrap()[0]["diagram"]["nodes"][0]["x"],
        800
    );
}
#[test]
fn stale_save_does_not_overwrite_graph_and_missing_target_stays_unresolved() {
    let (_dir, mut s, w) = setup();
    let mut d = diagram();
    d["nodes"][0]["pageId"] = json!("missing-target");
    let old = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"自由图","blocks":[],"diagram":d}}),
        )
        .unwrap();
    let mut newer = old.clone();
    newer["diagram"]["nodes"][0]["x"] = json!(700);
    let saved = s
        .dispatch("save", json!({"workspaceId":w,"page":newer}))
        .unwrap();
    let mut stale = old.clone();
    stale["diagram"]["edges"][0]["label"] = json!("过期");
    assert!(s
        .dispatch("save", json!({"workspaceId":w,"page":stale}))
        .is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w})).unwrap()[0],
        saved
    );
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
        .any(|x| x.as_str().unwrap().contains("missing-target")));
    let restored = s.dispatch("pages", json!({"workspaceId":dest})).unwrap();
    assert_eq!(restored[0]["diagram"]["nodes"][0]["pageId"], "");
    assert_eq!(
        restored[0]["diagram"]["nodes"][0]["unresolvedPageId"],
        "missing-target"
    );
}
#[test]
fn invalid_remote_edges_are_rejected_before_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let remote = tempfile::tempdir().unwrap();
    git2::Repository::init_bare(remote.path()).unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = s
        .dispatch(
            "createWorkspace",
            json!({"name":"远程自由图","remote":remote.path()}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"自由图","blocks":[],"diagram":diagram()}}),
        )
        .unwrap();
    s.dispatch("sync", json!({"workspaceId":w})).unwrap();
    let copy = tempfile::tempdir().unwrap();
    let repo = git2::build::RepoBuilder::new()
        .branch("main")
        .clone(remote.path().to_str().unwrap(), copy.path())
        .unwrap();
    let mut bad = page.clone();
    bad["diagram"]["edges"][0]["targetId"] = json!("missing");
    std::fs::write(
        copy.path()
            .join(format!("pages/{}.json", page["id"].as_str().unwrap())),
        serde_json::to_vec(&bad).unwrap(),
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["pages"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = git2::Signature::now("测试", "local@test.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "无效节点图", &tree, &[&parent])
        .unwrap();
    repo.find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();
    assert!(s.dispatch("sync", json!({"workspaceId":w})).is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w})).unwrap()[0]["diagram"],
        page["diagram"]
    );
}
#[test]
fn malformed_backup_is_rejected_without_partial_import() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::io::{Cursor, Write};
    let (_dir, mut s, w) = setup();
    let before = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"已有页面","blocks":[]}}),
        )
        .unwrap();
    let mut bad = diagram();
    bad["edges"][0]["targetId"] = json!("absent");
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(br#"{"format":"qingjian","version":1}"#)
        .unwrap();
    zip.start_file("pages/valid.json", options).unwrap();
    zip.write_all(
        serde_json::to_string(&json!({"id":"valid","title":"正常","blocks":[]}))
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    zip.start_file("pages/bad.json", options).unwrap();
    zip.write_all(
        serde_json::to_string(&json!({"id":"bad","title":"非法图","blocks":[],"diagram":bad}))
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    assert!(s
        .dispatch(
            "import",
            json!({"workspaceId":w,"name":"backup.zip","data":STANDARD.encode(bytes)})
        )
        .is_err());
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1);
    assert_eq!(pages[0], before);
}
#[test]
fn unknown_version_two_mode_is_preserved_without_becoming_editable() {
    let (dir, mut s, w) = setup();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"未来图","blocks":[]}}),
        )
        .unwrap();
    let file = dir
        .path()
        .join("workspaces")
        .join(&w)
        .join("pages")
        .join(format!("{}.json", page["id"].as_str().unwrap()));
    let mut future = page.clone();
    future["diagram"] = json!({"schemaVersion":2,"mode":"future-kind","opaque":{"keep":true}});
    std::fs::write(&file, serde_json::to_vec(&future).unwrap()).unwrap();
    let read = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(read[0]["diagram"], future["diagram"]);
    let mut draft = read[0].clone();
    draft.as_object_mut().unwrap().remove("diagram");
    assert!(s
        .dispatch("save", json!({"workspaceId":w,"page":draft}))
        .is_err());
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(file).unwrap()).unwrap(),
        future
    );
}
#[test]
fn unknown_graph_backup_keeps_opaque_fields_and_reports_no_remapping() {
    let (dir, mut s, w) = setup();
    let target = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"目标","blocks":[]}}),
        )
        .unwrap();
    let page = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"未来","blocks":[]}}),
        )
        .unwrap();
    let mut future = page.clone();
    future["diagram"] =
        json!({"schemaVersion":3,"mode":"future","nodes":[{"pageId":target["id"],"other":"未知"}]});
    let file = dir
        .path()
        .join("workspaces")
        .join(&w)
        .join("pages")
        .join(format!("{}.json", page["id"].as_str().unwrap()));
    std::fs::write(file, serde_json::to_vec(&future).unwrap()).unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let dest = s
        .dispatch("createWorkspace", json!({"name":"恢复未来图"}))
        .unwrap()["id"]
        .clone();
    let report = s
        .dispatch(
            "import",
            json!({"workspaceId":dest,"name":"backup.zip","data":backup["data"]}),
        )
        .unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":dest})).unwrap();
    let restored = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "未来")
        .unwrap();
    assert_eq!(restored["diagram"], future["diagram"]);
    assert!(report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().unwrap().contains("未重映射")));
}
