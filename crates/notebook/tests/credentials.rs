use notebook_core::Store;
use serde_json::json;
#[test]
fn unsupported_secure_storage_must_not_claim_remembered() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::new(root.path()).unwrap();
    let w = store
        .dispatch(
            "createWorkspace",
            json!({"name":"匿名认证测试","remote":"https://gitee.com/test/notes.git"}),
        )
        .unwrap();
    let result=store.dispatch("credentials",json!({"workspaceId":w["id"],"username":"test-user","token":"test-only-secret","remember":true}));
    assert!(result.is_err(), "没有安全凭据库时不得假报已记住认证");
    assert!(!std::fs::read_to_string(root.path().join("config.json"))
        .unwrap()
        .contains("test-only-secret"));
}
use notebook_core::{CredentialVault, Result};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
#[derive(Default)]
struct Vault {
    values: Mutex<HashMap<String, (String, String)>>,
    fail: Mutex<bool>,
}
impl CredentialVault for Vault {
    fn get(&self, key: &str) -> Result<Option<(String, String)>> {
        if *self.fail.lock().unwrap() {
            return Err("测试凭据库已锁定".into());
        }
        Ok(self.values.lock().unwrap().get(key).cloned())
    }
    fn set(&self, key: &str, username: &str, token: &str) -> Result<()> {
        if *self.fail.lock().unwrap() {
            return Err("测试凭据库拒绝保存".into());
        }
        self.values
            .lock()
            .unwrap()
            .insert(key.into(), (username.into(), token.into()));
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<()> {
        if *self.fail.lock().unwrap() {
            return Err("测试凭据库拒绝删除".into());
        }
        self.values.lock().unwrap().remove(key);
        Ok(())
    }
}
fn create(store: &mut Store) -> serde_json::Value {
    store
        .dispatch(
            "createWorkspace",
            json!({"name":"匿名测试","remote":"https://gitee.com/test/notes.git"}),
        )
        .unwrap()
}
fn save(store: &mut Store, w: &serde_json::Value, remember: bool) -> Result<serde_json::Value> {
    store.dispatch("credentials",json!({"workspaceId":w["id"],"username":"test-user","token":"test-only-secret","remember":remember}))
}
#[test]
fn remembered_account_survives_restart_without_secret_in_files_or_responses() {
    let root = tempfile::tempdir().unwrap();
    let vault = Arc::new(Vault::default());
    let mut store = Store::new(root.path()).unwrap();
    store.set_credential_vault(vault.clone());
    let w = create(&mut store);
    let status = save(&mut store, &w, true).unwrap();
    assert_eq!(status["saved"], true);
    assert!(!status.to_string().contains("test-only-secret"));
    drop(store);
    let mut restarted = Store::new(root.path()).unwrap();
    restarted.set_credential_vault(vault);
    for _ in 0..2 {
        let status = restarted
            .dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
            .unwrap();
        assert_eq!(status["username"], "test-user");
        assert_eq!(status["saved"], true);
    }
    for entry in walkdir::WalkDir::new(root.path())
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        assert!(
            !String::from_utf8_lossy(&std::fs::read(entry.path()).unwrap())
                .contains("test-only-secret")
        );
    }
    assert!(restarted
        .dispatch(
            "credentials",
            json!({"workspaceId":w["id"],"username":"test-user","token":"","remember":true})
        )
        .is_ok());
    assert!(restarted
        .dispatch(
            "credentials",
            json!({"workspaceId":w["id"],"username":"different-user","token":"","remember":true})
        )
        .is_err());
}
#[test]
fn session_credentials_are_not_restored_after_restart() {
    let root = tempfile::tempdir().unwrap();
    let vault = Arc::new(Vault::default());
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault.clone());
    let w = create(&mut s);
    save(&mut s, &w, false).unwrap();
    drop(s);
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault.clone());
    let status = s
        .dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
        .unwrap();
    assert_eq!(status["session"], false);
    assert!(vault.values.lock().unwrap().is_empty());
}
#[test]
fn credential_scope_is_workspace_and_remote_and_rename_keeps_auth() {
    let root = tempfile::tempdir().unwrap();
    let vault = Arc::new(Vault::default());
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault.clone());
    let mut w = create(&mut s);
    save(&mut s, &w, true).unwrap();
    let other = create(&mut s);
    assert_eq!(
        s.dispatch("credentialStatus", json!({"workspaceId":other["id"]}))
            .unwrap()["saved"],
        false
    );
    w["name"] = json!("改名");
    s.dispatch("updateWorkspace", json!({"workspace":w}))
        .unwrap();
    assert_eq!(
        s.dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
            .unwrap()["saved"],
        true
    );
    w["remote"] = json!("https://gitee.com/test/other.git");
    s.dispatch("updateWorkspace", json!({"workspace":w}))
        .unwrap();
    assert!(vault.values.lock().unwrap().is_empty());
    assert_eq!(
        s.dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
            .unwrap()["session"],
        false
    );
}
#[test]
fn forgetting_and_removal_delete_secure_entry() {
    let root = tempfile::tempdir().unwrap();
    let vault = Arc::new(Vault::default());
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault.clone());
    let w = create(&mut s);
    save(&mut s, &w, true).unwrap();
    s.dispatch("forgetCredentials", json!({"workspaceId":w["id"]}))
        .unwrap();
    assert!(vault.values.lock().unwrap().is_empty());
    save(&mut s, &w, true).unwrap();
    let preview = s
        .dispatch("removeWorkspacePreview", json!({"workspaceId":w["id"]}))
        .unwrap();
    s.dispatch(
        "removeWorkspace",
        json!({"workspaceId":w["id"],"preview":preview,"confirmed":true}),
    )
    .unwrap();
    assert!(vault.values.lock().unwrap().is_empty());
}
#[test]
fn vault_failure_does_not_claim_saved_or_allow_remote_change() {
    let root = tempfile::tempdir().unwrap();
    let vault = Arc::new(Vault::default());
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault.clone());
    let mut w = create(&mut s);
    *vault.fail.lock().unwrap() = true;
    assert!(save(&mut s, &w, true).is_err());
    assert_eq!(
        s.dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
            .unwrap()["saved"],
        false
    );
    *vault.fail.lock().unwrap() = false;
    save(&mut s, &w, true).unwrap();
    *vault.fail.lock().unwrap() = true;
    w["remote"] = json!("https://github.com/test/other.git");
    assert!(s
        .dispatch("updateWorkspace", json!({"workspace":w}))
        .is_err());
    assert_eq!(
        s.dispatch("bootstrap", json!({})).unwrap()["workspaces"][0]["remote"],
        "https://gitee.com/test/notes.git"
    );
    drop(s);
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault);
    assert!(s
        .dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
        .unwrap_err()
        .contains("锁定"));
}
#[test]
fn configuration_failure_does_not_leave_false_saved_status() {
    let root = tempfile::tempdir().unwrap();
    let vault = Arc::new(Vault::default());
    let mut s = Store::new(root.path()).unwrap();
    s.set_credential_vault(vault.clone());
    let w = create(&mut s);
    let config = root.path().join("config.json");
    std::fs::rename(&config, root.path().join("config.backup")).unwrap();
    std::fs::create_dir(&config).unwrap();
    assert!(save(&mut s, &w, true).is_err());
    assert_eq!(
        s.dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
            .unwrap()["saved"],
        false
    );
    assert!(vault.values.lock().unwrap().is_empty());
}
