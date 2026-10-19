use notebook_core::Store;
use serde_json::{json, Value};
use tempfile::TempDir;
fn call(s: &mut Store, a: &str, q: Value) -> Value {
    s.dispatch(a, q).unwrap()
}
fn save(s: &mut Store, w: &str, id: &str, title: &str) -> Value {
    let pages = call(s, "pages", json!({"workspaceId":w}));
    let mut p = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == id)
        .cloned()
        .unwrap_or(json!({"id":id,"blocks":[]}));
    p["title"] = json!(title);
    call(s, "save", json!({"workspaceId":w,"page":p}))
}
fn setup() -> (TempDir, Store, String, String) {
    let t = TempDir::new().unwrap();
    let r = t.path().join("remote.git");
    git2::Repository::init_bare(&r).unwrap();
    let mut s = Store::new(&t.path().join("data")).unwrap();
    let a = call(
        &mut s,
        "createWorkspace",
        json!({"name":"设备甲","remote":r}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    save(&mut s, &a, "one", "原始内容");
    call(&mut s, "sync", json!({"workspaceId":a}));
    let b = call(
        &mut s,
        "createWorkspace",
        json!({"name":"设备乙","remote":r,"mode":"clone"}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    (t, s, a, b)
}
#[test]
fn independent_pages_merge_without_external_git() {
    let (_t, mut s, a, b) = setup();
    save(&mut s, &a, "two", "甲新增");
    save(&mut s, &b, "three", "乙新增");
    call(&mut s, "sync", json!({"workspaceId":a}));
    let r = call(&mut s, "sync", json!({"workspaceId":b}));
    assert_eq!(r["status"], "synced");
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":b}))
            .as_array()
            .unwrap()
            .len(),
        3
    );
    call(&mut s, "sync", json!({"workspaceId":a}));
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":a}))
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
#[test]
fn same_page_choice_keeps_history_and_recoverable_backup() {
    let (t, mut s, a, b) = setup();
    save(&mut s, &a, "one", "远程修改");
    save(&mut s, &b, "one", "本机修改");
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    assert_eq!(p["status"], "conflict");
    assert_eq!(p["conflicts"][0]["local"]["title"], "本机修改");
    assert!(s
        .dispatch("resolveSync", json!({"workspaceId":b,"preview":p}))
        .is_err());
    let r = call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"choices":{"pages/one.json":"remote"}}),
    );
    assert_eq!(r["status"], "synced");
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":b}))[0]["title"],
        "远程修改"
    );
    let repo = git2::Repository::open(t.path().join("data/workspaces").join(&b)).unwrap();
    assert_eq!(
        repo.head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .parent_count(),
        2
    );
    let backups = call(&mut s, "syncBackups", json!({"workspaceId":b}));
    assert!(!backups.as_array().unwrap().is_empty());
    let restored = call(
        &mut s,
        "restoreSyncBackup",
        json!({"workspaceId":b,"backupId":backups[0]["id"],"side":"local"}),
    );
    assert_eq!(restored["remote"], "");
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":restored["id"]}))[0]["title"],
        "本机修改"
    );
    assert_eq!(
        call(&mut s, "sync", json!({"workspaceId":b}))["status"],
        "synced"
    );
}
#[test]
fn unrelated_history_requires_choice_and_old_preview_is_rejected() {
    let (_t, mut s, a, _b) = setup();
    let remote = call(&mut s, "bootstrap", json!({}))["workspaces"][0]["remote"].clone();
    let c = call(
        &mut s,
        "createWorkspace",
        json!({"name":"新手机","remote":remote}),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    save(&mut s, &c, "phone", "手机默认页面");
    let p = call(&mut s, "sync", json!({"workspaceId":c}));
    assert_eq!(p["unrelated"], true);
    save(&mut s, &a, "one", "远程再次修改");
    call(&mut s, "sync", json!({"workspaceId":a}));
    assert!(s
        .dispatch(
            "resolveSync",
            json!({"workspaceId":c,"preview":p,"strategy":"remote"})
        )
        .unwrap_err()
        .contains("变化"));
    let p = call(&mut s, "sync", json!({"workspaceId":c}));
    save(&mut s, &c, "phone", "本机再次修改");
    assert!(s
        .dispatch(
            "resolveSync",
            json!({"workspaceId":c,"preview":p,"strategy":"remote"})
        )
        .unwrap_err()
        .contains("变化"));
    let p = call(&mut s, "sync", json!({"workspaceId":c}));
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":c,"preview":p,"strategy":"remote"}),
    );
    let pages = call(&mut s, "pages", json!({"workspaceId":c}));
    assert_eq!(pages.as_array().unwrap().len(), 1);
    assert_eq!(pages[0]["title"], "远程再次修改");
}
#[test]
fn workspace_remove_restore_and_last_workspace() {
    let t = TempDir::new().unwrap();
    let mut s = Store::new(t.path()).unwrap();
    let w = call(&mut s, "createWorkspace", json!({"name":"待移除工作区"}));
    let id = w["id"].as_str().unwrap();
    save(&mut s, id, "note", "重要笔记");
    let preview = call(&mut s, "removeWorkspacePreview", json!({"workspaceId":id}));
    assert_eq!(preview["pageCount"], 1);
    assert!(s
        .dispatch(
            "removeWorkspace",
            json!({"workspaceId":id,"preview":preview})
        )
        .is_err());
    save(&mut s, id, "note", "预览后修改");
    assert!(s
        .dispatch(
            "removeWorkspace",
            json!({"workspaceId":id,"preview":preview,"confirmed":true})
        )
        .unwrap_err()
        .contains("变化"));
    let preview = call(&mut s, "removeWorkspacePreview", json!({"workspaceId":id}));
    call(
        &mut s,
        "removeWorkspace",
        json!({"workspaceId":id,"preview":preview,"confirmed":true}),
    );
    let config = call(&mut s, "bootstrap", json!({}));
    assert_eq!(config["workspaces"].as_array().unwrap().len(), 1);
    assert_ne!(config["activeId"], id);
    assert!(
        call(&mut s, "pages", json!({"workspaceId":config["activeId"]}))
            .as_array()
            .unwrap()
            .is_empty()
    );
    let archives = call(&mut s, "archivedWorkspaces", json!({}));
    let archive = archives[0]["id"].clone();
    assert!(s
        .dispatch("purgeWorkspace", json!({"archiveId":archive}))
        .is_err());
    let restored = call(&mut s, "restoreWorkspace", json!({"archiveId":archive}));
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":restored["id"]}))[0]["title"],
        "预览后修改"
    );
}

#[test]
fn cross_page_parent_cycle_is_a_resolvable_conflict() {
    let (_t, mut s, a, b) = setup();
    save(&mut s, &a, "two", "另一页");
    call(&mut s, "sync", json!({"workspaceId":a}));
    call(&mut s, "sync", json!({"workspaceId":b}));
    let mut pa = call(&mut s, "pages", json!({"workspaceId":a}))
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "one")
        .unwrap()
        .clone();
    pa["parentId"] = json!("two");
    call(&mut s, "save", json!({"workspaceId":a,"page":pa}));
    let mut pb = call(&mut s, "pages", json!({"workspaceId":b}))
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "two")
        .unwrap()
        .clone();
    pb["parentId"] = json!("one");
    call(&mut s, "save", json!({"workspaceId":b,"page":pb}));
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    assert_eq!(p["status"], "conflict");
    assert!(p["structureError"].is_string());
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"strategy":"remote"}),
    );
}
#[test]
fn resolved_remote_version_rejects_stale_window_revision() {
    let (_t, mut s, a, b) = setup();
    save(&mut s, &a, "one", "远程修改");
    let stale = save(&mut s, &b, "one", "本机修改");
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"strategy":"merge","choices":{"pages/one.json":"remote"}}),
    );
    assert!(s
        .dispatch("save", json!({"workspaceId":b,"page":stale}))
        .is_err());
}
#[test]
fn same_page_can_choose_whole_local_workspace_and_remote_backup() {
    let (_t, mut s, a, b) = setup();
    save(&mut s, &a, "one", "远程修改");
    save(&mut s, &b, "one", "本机修改");
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"strategy":"local"}),
    );
    call(&mut s, "sync", json!({"workspaceId":a}));
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":a}))[0]["title"],
        "本机修改"
    );
    let backups = call(&mut s, "syncBackups", json!({"workspaceId":b}));
    let restored = call(
        &mut s,
        "restoreSyncBackup",
        json!({"workspaceId":b,"backupId":backups[0]["id"],"side":"remote"}),
    );
    assert_eq!(
        call(&mut s, "pages", json!({"workspaceId":restored["id"]}))[0]["title"],
        "远程修改"
    );
}

#[test]
fn keep_modified_page_also_keeps_its_remote_deleted_attachment() {
    let (_t, mut s, a, b) = setup();
    let attachment = call(
        &mut s,
        "attach",
        json!({"workspaceId":a,"name":"image.png","data":"aGVsbG8="}),
    );
    let name = attachment["name"].as_str().unwrap();
    let mut p = call(&mut s, "pages", json!({"workspaceId":a}))[0].clone();
    p["blocks"] = json!([{"id":"image","type":"image","text":format!("asset:{name}")}]);
    call(&mut s, "save", json!({"workspaceId":a,"page":p}));
    call(&mut s, "sync", json!({"workspaceId":a}));
    call(&mut s, "sync", json!({"workspaceId":b}));
    let mut local = call(&mut s, "pages", json!({"workspaceId":b}))[0].clone();
    local["title"] = json!("本机仍需此附件");
    call(&mut s, "save", json!({"workspaceId":b,"page":local}));
    call(
        &mut s,
        "trash",
        json!({"workspaceId":a,"id":"one","deleted":true}),
    );
    let preview = call(&mut s, "purgePreview", json!({"workspaceId":a,"all":true}));
    call(
        &mut s,
        "purgeTrash",
        json!({"workspaceId":a,"preview":preview,"confirmed":true}),
    );
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"strategy":"merge","choices":{"pages/one.json":"local"}}),
    );
    assert_eq!(
        call(&mut s, "asset", json!({"workspaceId":b,"name":name}))["data"],
        "aGVsbG8="
    );
}
#[test]
fn choosing_deleted_page_rejects_stale_window_save() {
    let (_t, mut s, a, b) = setup();
    let stale = save(&mut s, &b, "one", "手机持有旧草稿");
    call(
        &mut s,
        "trash",
        json!({"workspaceId":a,"id":"one","deleted":true}),
    );
    let preview = call(&mut s, "purgePreview", json!({"workspaceId":a,"all":true}));
    call(
        &mut s,
        "purgeTrash",
        json!({"workspaceId":a,"preview":preview,"confirmed":true}),
    );
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"strategy":"merge","choices":{"pages/one.json":"remote"}}),
    );
    assert!(s
        .dispatch("save", json!({"workspaceId":b,"page":stale}))
        .is_err());
}

#[test]
fn local_choice_advances_revision_for_the_other_devices_old_window() {
    let (_t, mut s, a, b) = setup();
    let stale = save(&mut s, &a, "one", "电脑旧草稿");
    save(&mut s, &b, "one", "手机保留内容");
    call(&mut s, "sync", json!({"workspaceId":a}));
    let p = call(&mut s, "sync", json!({"workspaceId":b}));
    call(
        &mut s,
        "resolveSync",
        json!({"workspaceId":b,"preview":p,"strategy":"local"}),
    );
    call(&mut s, "sync", json!({"workspaceId":a}));
    assert!(s
        .dispatch("save", json!({"workspaceId":a,"page":stale}))
        .is_err());
}

#[test]
fn database_conflict_previews_show_changed_cell_values() {
    let (_t, mut s, a, b) = setup();
    let mut p = call(&mut s, "pages", json!({"workspaceId":a}))[0].clone();
    p["database"] = json!({"columns":[{"id":"title","name":"电影","type":"text"},{"id":"status","name":"观看状态","type":"text"}],"rows":[{"id":"movie","values":{"title":"入青云","status":"未观看"}}]});
    call(&mut s, "save", json!({"workspaceId":a,"page":p}));
    call(&mut s, "sync", json!({"workspaceId":a}));
    call(&mut s, "sync", json!({"workspaceId":b}));
    let mut x = call(&mut s, "pages", json!({"workspaceId":a}))[0].clone();
    let mut y = x.clone();
    x["database"]["rows"][0]["values"]["status"] = json!("观看中");
    y["database"]["rows"][0]["values"]["status"] = json!("已看完");
    call(&mut s, "save", json!({"workspaceId":a,"page":x}));
    call(&mut s, "save", json!({"workspaceId":b,"page":y}));
    call(&mut s, "sync", json!({"workspaceId":a}));
    let preview = call(&mut s, "sync", json!({"workspaceId":b}));
    assert_eq!(
        preview["conflicts"][0]["local"]["database"]["cells"][0]["value"],
        "已看完"
    );
    assert_eq!(
        preview["conflicts"][0]["remote"]["database"]["cells"][0]["value"],
        "观看中"
    );
}

#[test]
fn restoring_purged_history_does_not_reuse_the_first_revision() {
    let (_t, mut s, a, _b) = setup();
    let stale = call(&mut s, "pages", json!({"workspaceId":a}))[0].clone();
    save(&mut s, &a, "one", "第二版需恢复");
    let saved = call(
        &mut s,
        "commit",
        json!({"workspaceId":a,"message":"第二版"}),
    );
    call(
        &mut s,
        "trash",
        json!({"workspaceId":a,"id":"one","deleted":true}),
    );
    let preview = call(&mut s, "purgePreview", json!({"workspaceId":a,"all":true}));
    call(
        &mut s,
        "purgeTrash",
        json!({"workspaceId":a,"preview":preview,"confirmed":true}),
    );
    call(
        &mut s,
        "restore",
        json!({"workspaceId":a,"id":"one","commit":saved["commit"]}),
    );
    assert!(s
        .dispatch("save", json!({"workspaceId":a,"page":stale}))
        .is_err());
}

#[test]
fn nonempty_remote_with_different_branch_does_not_silently_upload_new_branch() {
    let t = TempDir::new().unwrap();
    let remote = t.path().join("remote.git");
    let repo = git2::Repository::init_bare(&remote).unwrap();
    let mut s = Store::new(&t.path().join("data")).unwrap();
    let a = call(
        &mut s,
        "createWorkspace",
        json!({"name":"已有分支","remote":remote,"branch":"master"}),
    );
    save(&mut s, a["id"].as_str().unwrap(), "one", "远程原有内容");
    call(&mut s, "sync", json!({"workspaceId":a["id"]}));
    let b = call(
        &mut s,
        "createWorkspace",
        json!({"name":"新手机","remote":remote,"branch":"main"}),
    );
    save(&mut s, b["id"].as_str().unwrap(), "two", "本机内容");
    let message = s
        .dispatch("sync", json!({"workspaceId":b["id"]}))
        .unwrap_err();
    assert!(message.contains("master"));
    assert!(repo.find_reference("refs/heads/main").is_err());
}
