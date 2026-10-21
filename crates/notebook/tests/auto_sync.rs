use notebook_core::Store;
use serde_json::{json, Value};
fn setup() -> (tempfile::TempDir, Store, Value, Value) {
    let root = tempfile::tempdir().unwrap();
    let mut s = Store::new(root.path()).unwrap();
    let w = s
        .dispatch("createWorkspace", json!({"name":"匿名自动同步"}))
        .unwrap();
    let p = s
        .dispatch(
            "save",
            json!({"workspaceId":w["id"],"page":{"title":"原文","blocks":[]}}),
        )
        .unwrap();
    (root, s, w, p)
}
#[test]
fn auto_sync_setting_is_per_workspace_and_persistent() {
    let (root, mut s, w, _) = setup();
    s.dispatch(
        "autoSyncSettings",
        json!({"workspaceId":w["id"],"enabled":false}),
    )
    .unwrap();
    assert!(s
        .dispatch(
            "autoSyncSettings",
            json!({"workspaceId":w["id"],"enabled":"false"})
        )
        .is_err());
    drop(s);
    let mut s = Store::new(root.path()).unwrap();
    assert_eq!(
        s.dispatch("bootstrap", json!({})).unwrap()["workspaces"][0]["autoSync"],
        false
    );
}
#[test]
fn concurrent_draft_keeps_both_snapshots_and_rejects_stale_choice() {
    let (root, mut s, w, p) = setup();
    let mut draft = p.clone();
    draft["title"] = json!("网络中继续输入");
    let mut remote = p.clone();
    remote["title"] = json!("已下载远端内容");
    let remote = s
        .dispatch("save", json!({"workspaceId":w["id"],"page":remote}))
        .unwrap();
    let preview = s
        .dispatch(
            "syncDraftPreview",
            json!({"workspaceId":w["id"],"draft":draft}),
        )
        .unwrap();
    assert_eq!(preview["remote"], remote);
    assert_eq!(preview["draft"], draft);
    assert!(root
        .path()
        .join("sync-draft-backups")
        .join(w["id"].as_str().unwrap())
        .join(format!("{}.json", preview["id"].as_str().unwrap()))
        .exists());
    let mut changed = remote;
    changed["title"] = json!("另一窗口又修改");
    s.dispatch("save", json!({"workspaceId":w["id"],"page":changed}))
        .unwrap();
    assert!(s
        .dispatch(
            "resolveSyncDraft",
            json!({"workspaceId":w["id"],"backupId":preview["id"],"choice":"local"})
        )
        .is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w["id"]})).unwrap()[0]["title"],
        "另一窗口又修改"
    );
}
#[test]
fn choosing_local_advances_revision_and_remote_choice_preserves_local_snapshot() {
    let (_root, mut s, w, p) = setup();
    let mut draft = p.clone();
    draft["title"] = json!("本机草稿");
    let preview = s
        .dispatch(
            "syncDraftPreview",
            json!({"workspaceId":w["id"],"draft":draft}),
        )
        .unwrap();
    let result = s
        .dispatch(
            "resolveSyncDraft",
            json!({"workspaceId":w["id"],"backupId":preview["id"],"choice":"local"}),
        )
        .unwrap();
    assert!(result["page"]["revision"].as_u64().unwrap() > p["revision"].as_u64().unwrap());
    let copy = s
        .dispatch(
            "restoreSyncDraft",
            json!({"workspaceId":w["id"],"backupId":preview["id"]}),
        )
        .unwrap();
    assert_ne!(copy["id"], p["id"]);
    assert_eq!(copy["parentId"], "");
    assert!(s
        .dispatch(
            "resolveSyncDraft",
            json!({"workspaceId":w["id"],"backupId":"../escape","choice":"remote"})
        )
        .is_err());
}

#[test]
fn draft_backup_preserves_attachments_removed_by_download() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (root, mut s, w, mut p) = setup();
    let asset = s.dispatch("attach", json!({"workspaceId":w["id"],"name":"附件.txt","data":STANDARD.encode(b"draft attachment")})).unwrap();
    let name = asset["name"].as_str().unwrap();
    p["blocks"] =
        json!([{"id":"attachment","type":"paragraph","text":format!("[附件](asset:{name})")}]);
    p = s
        .dispatch("save", json!({"workspaceId":w["id"],"page":p}))
        .unwrap();
    let source = s
        .dispatch("commit", json!({"workspaceId":w["id"]}))
        .unwrap();
    let path = root
        .path()
        .join("workspaces")
        .join(w["id"].as_str().unwrap())
        .join("assets")
        .join(name);
    std::fs::remove_file(&path).unwrap();
    let preview = s
        .dispatch(
            "syncDraftPreview",
            json!({"workspaceId":w["id"],"draft":p,"sourceCommit":source["commit"]}),
        )
        .unwrap();
    assert!(!path.exists(), "预览不能把已删除的附件重新放回同步仓库");
    let copy = s
        .dispatch(
            "restoreSyncDraft",
            json!({"workspaceId":w["id"],"backupId":preview["id"]}),
        )
        .unwrap();
    assert_ne!(copy["id"], p["id"]);
    assert_eq!(std::fs::read(&path).unwrap(), b"draft attachment");
    std::fs::remove_file(&path).unwrap();
    s.dispatch(
        "restoreSyncDraft",
        json!({"workspaceId":w["id"],"backupId":preview["id"]}),
    )
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"draft attachment");
}

#[test]
fn missing_attachment_cannot_produce_a_false_recovery_point() {
    let (_root, mut s, w, mut p) = setup();
    p["blocks"] =
        json!([{"id":"attachment","type":"paragraph","text":"[附件](asset:missing.txt)"}]);
    assert!(s
        .dispatch("syncDraftPreview", json!({"workspaceId":w["id"],"draft":p}))
        .is_err());
}

#[test]
fn downloaded_same_name_attachment_cannot_replace_the_draft_original() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (root, mut s, w, mut p) = setup();
    let asset = s
        .dispatch(
            "attach",
            json!({"workspaceId":w["id"],"name":"附件.txt","data":STANDARD.encode(b"original")}),
        )
        .unwrap();
    let name = asset["name"].as_str().unwrap();
    p["blocks"] =
        json!([{"id":"attachment","type":"paragraph","text":format!("[附件](asset:{name})")}]);
    p = s
        .dispatch("save", json!({"workspaceId":w["id"],"page":p}))
        .unwrap();
    let source = s
        .dispatch("commit", json!({"workspaceId":w["id"]}))
        .unwrap();
    let path = root
        .path()
        .join("workspaces")
        .join(w["id"].as_str().unwrap())
        .join("assets")
        .join(name);
    std::fs::write(&path, b"downloaded replacement").unwrap();
    let preview = s
        .dispatch(
            "syncDraftPreview",
            json!({"workspaceId":w["id"],"draft":p,"sourceCommit":source["commit"]}),
        )
        .unwrap();
    assert!(
        s.dispatch(
            "restoreSyncDraft",
            json!({"workspaceId":w["id"],"backupId":preview["id"]})
        )
        .is_err(),
        "同名新附件不能被覆盖或伪装为原始附件"
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"downloaded replacement");
    std::fs::remove_file(&path).unwrap();
    s.dispatch(
        "restoreSyncDraft",
        json!({"workspaceId":w["id"],"backupId":preview["id"]}),
    )
    .unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"original");
}

#[test]
fn permanent_workspace_cleanup_removes_its_draft_backups_only() {
    let (root, mut s, w, p) = setup();
    s.dispatch("syncDraftPreview", json!({"workspaceId":w["id"],"draft":p}))
        .unwrap();
    let other = s
        .dispatch("createWorkspace", json!({"name":"保留的工作区"}))
        .unwrap();
    let other_page = s
        .dispatch(
            "save",
            json!({"workspaceId":other["id"],"page":{"title":"保留正文","blocks":[]}}),
        )
        .unwrap();
    s.dispatch(
        "syncDraftPreview",
        json!({"workspaceId":other["id"],"draft":other_page}),
    )
    .unwrap();
    let preview = s
        .dispatch("removeWorkspacePreview", json!({"workspaceId":w["id"]}))
        .unwrap();
    s.dispatch(
        "removeWorkspace",
        json!({"workspaceId":w["id"],"confirmed":true,"preview":preview}),
    )
    .unwrap();
    let backups = root.path().join("sync-draft-backups");
    assert!(backups.join(w["id"].as_str().unwrap()).exists());
    let archives = s.dispatch("archivedWorkspaces", json!({})).unwrap();
    let archive = &archives[0];
    s.dispatch(
        "purgeWorkspace",
        json!({"archiveId":archive["id"],"confirmed":true,"fingerprint":archive["fingerprint"]}),
    )
    .unwrap();
    assert!(!backups.join(w["id"].as_str().unwrap()).exists());
    assert!(backups.join(other["id"].as_str().unwrap()).exists());
}
