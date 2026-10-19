use crate::{json, text, Result, Store, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// 系统实现只保存密钥，不得使用笔记目录或明文文件替代。
pub trait CredentialVault: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<(String, String)>>;
    fn set(&self, key: &str, username: &str, token: &str) -> Result<()>;
    fn delete(&self, key: &str) -> Result<()>;
}
impl Store {
    pub fn set_credential_vault(&mut self, vault: Arc<dyn CredentialVault>) {
        self.credential_vault = Some(vault);
    }
    fn credential_key(&self, w: &Value) -> Result<String> {
        let root = std::fs::canonicalize(&self.root).map_err(crate::err)?;
        let scope =
            serde_json::to_vec(&(root, text(w, "id"), text(w, "remote"))).map_err(crate::err)?;
        Ok(format!("{:x}", Sha256::digest(scope)))
    }
    pub(crate) fn load_credentials(&mut self, w: &Value) -> Result<(String, String)> {
        let id = text(w, "id");
        if let Some(auth) = self.credentials.get(id) {
            return Ok(auth.clone());
        }
        if self.config["credentialScopes"][id].as_str() == Some(text(w, "remote")) {
            let vault = self
                .credential_vault
                .as_ref()
                .ok_or("此环境不能读取系统安全认证，请在原生应用中打开，或改用本次会话认证")?;
            if let Some(auth) = vault.get(&self.credential_key(w)?)? {
                self.credentials.insert(id.into(), auth.clone());
                return Ok(auth);
            }
        }
        Ok(Default::default())
    }
    pub(crate) fn credential_status(&mut self, id: &str) -> Result<Value> {
        let (w, _) = self.workspace(id)?;
        let available = self.credential_vault.is_some();
        let auth = self.load_credentials(&w)?;
        Ok(
            json!({"available":available,"saved":!auth.1.is_empty() && self.config["credentialScopes"][id].as_str()==Some(text(&w,"remote")),"session":!auth.1.is_empty(),"username":auth.0}),
        )
    }
    pub(crate) fn clear_credentials(&mut self, w: &Value) -> Result<()> {
        let id = text(w, "id");
        if self.config["credentialScopes"][id].is_string() {
            let vault = self
                .credential_vault
                .as_ref()
                .ok_or("无法清除系统认证，请在原生应用中完成此操作")?;
            vault.delete(&self.credential_key(w)?)?;
            if let Some(scopes) = self.config["credentialScopes"].as_object_mut() {
                scopes.remove(id);
            }
            self.persist()?;
        }
        self.credentials.remove(id);
        Ok(())
    }
    pub(crate) fn save_credentials(&mut self, q: &Value) -> Result<Value> {
        let id = text(q, "workspaceId");
        let (w, _) = self.workspace(id)?;
        if text(&w, "remote").is_empty() {
            return Err("请先关联远程仓库，再保存认证".into());
        }
        let remember = q["remember"] == true || q["remember"] == "on";
        if remember && self.credential_vault.is_none() {
            return Err("此环境不支持系统安全存储，请仅在本次会话使用认证".into());
        }
        let mut username = text(q, "username").trim().to_owned();
        let mut token = text(q, "token").trim().to_owned();
        if token.is_empty() {
            let previous = self.load_credentials(&w)?;
            if !username.is_empty() && username != previous.0 {
                return Err("更换用户名时请同时填写新的访问令牌".into());
            }
            if username.is_empty() {
                username = previous.0;
            }
            token = previous.1;
        }
        if token.is_empty() {
            return Err("请填写访问令牌".into());
        }
        if url::Url::parse(text(&w, "remote"))
            .ok()
            .and_then(|u| u.host_str().map(str::to_owned))
            .as_deref()
            == Some("gitee.com")
            && username.is_empty()
        {
            return Err("Gitee 认证需要填写个人主页中的真实用户名".into());
        }
        if remember {
            let key = self.credential_key(&w)?;
            let vault = self.credential_vault.as_ref().unwrap().clone();
            let previous = vault.get(&key)?;
            vault.set(&key, &username, &token)?;
            let before = self.config.clone();
            if !self.config["credentialScopes"].is_object() {
                self.config["credentialScopes"] = json!({});
            }
            self.config["credentialScopes"][id] = w["remote"].clone();
            if let Err(error) = self.persist() {
                self.config = before;
                let restored = if let Some((user, secret)) = previous {
                    vault.set(&key, &user, &secret)
                } else {
                    vault.delete(&key)
                };
                return Err(if restored.is_ok() {
                    error
                } else {
                    format!("{error}；系统认证回滚未完成，请在系统凭据库中清除此工作区认证后重试")
                });
            }
        } else {
            self.clear_credentials(&w)?;
        }
        self.credentials.insert(id.into(), (username, token));
        self.credential_status(id)
    }
}
