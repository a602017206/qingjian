use keyring_core::{Entry, Error};
use notebook_core::{CredentialVault, Result};

pub struct SystemVault {}
impl SystemVault {
    pub fn new() -> Self {
        Self {}
    }
    fn entry(&self, key: &str) -> Result<Entry> {
        if keyring_core::get_default_store().is_none() {
            #[cfg(target_os = "macos")]
            let store = apple_native_keyring_store::keychain::Store::new();
            #[cfg(target_os = "ios")]
            let store = apple_native_keyring_store::protected::Store::new();
            #[cfg(target_os = "windows")]
            let store = windows_native_keyring_store::Store::new();
            #[cfg(target_os = "linux")]
            let store = zbus_secret_service_keyring_store::Store::new();
            #[cfg(target_os = "android")]
            let store = android_native_keyring_store::Store::new_with_configuration(
                &std::collections::HashMap::from([("name", "qingjian-sync")]),
            );
            keyring_core::set_default_store(store.map_err(|_| {
                "系统安全凭据库不可用，请解锁系统凭据库后重试，或取消记住认证".to_owned()
            })?);
        }
        Entry::new("com.qingjian.notebook.sync", key).map_err(|_| "无法创建系统安全认证条目".into())
    }
}
impl CredentialVault for SystemVault {
    fn get(&self, key: &str) -> Result<Option<(String, String)>> {
        match self.entry(key)?.get_password() {
            Ok(value) => serde_json::from_str(&value)
                .map(Some)
                .map_err(|_| "系统保存的认证格式无效，请忘记认证后重新填写".into()),
            Err(Error::NoEntry) => Ok(None),
            Err(_) => Err("无法读取系统保存的认证，请解锁或允许访问系统凭据库后重试".into()),
        }
    }
    fn set(&self, key: &str, username: &str, token: &str) -> Result<()> {
        let value = serde_json::to_string(&(username, token)).map_err(|_| "认证格式无效")?;
        self.entry(key)?.set_password(&value).map_err(|_| {
            "认证未能安全保存，请允许访问系统凭据库后重试；也可以取消记住，仅在本次运行使用".into()
        })
    }
    fn delete(&self, key: &str) -> Result<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(_) => Err("无法清除系统保存的认证，请允许访问系统凭据库后重试".into()),
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use notebook_core::Store;
    use serde_json::json;
    use std::sync::Arc;
    #[test]
    #[ignore = "需要允许访问本机钥匙串，仅创建和删除匿名测试条目"]
    fn system_keychain_remembers_across_store_restart() {
        let root = tempfile::tempdir().unwrap();
        let vault = Arc::new(SystemVault {});
        let mut store = Store::new(root.path()).unwrap();
        store.set_credential_vault(vault.clone());
        let w = store.dispatch("createWorkspace", json!({"name":"匿名钥匙串验证","remote":"https://gitee.com/anonymous-test/notes.git"})).unwrap();
        struct Cleanup<'a>(&'a std::path::Path, String, Arc<SystemVault>);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                if let Ok(mut s) = Store::new(self.0) {
                    s.set_credential_vault(self.2.clone());
                    let _ = s.dispatch("forgetCredentials", json!({"workspaceId":self.1}));
                }
            }
        }
        let _cleanup = Cleanup(root.path(), w["id"].as_str().unwrap().into(), vault.clone());
        assert_eq!(store.dispatch("credentials",json!({"workspaceId":w["id"],"username":"anonymous-test-user","token":"anonymous-test-value","remember":true})).unwrap()["saved"], true);
        drop(store);
        let mut store = Store::new(root.path()).unwrap();
        store.set_credential_vault(vault);
        let status = store
            .dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
            .unwrap();
        assert_eq!(status["username"], "anonymous-test-user");
        assert_eq!(status["saved"], true);
        assert!(!status.to_string().contains("anonymous-test-value"));
        store
            .dispatch("forgetCredentials", json!({"workspaceId":w["id"]}))
            .unwrap();
        assert_eq!(
            store
                .dispatch("credentialStatus", json!({"workspaceId":w["id"]}))
                .unwrap()["saved"],
            false
        );
    }
}
