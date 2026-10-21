use crate::{ensure_dir, err, now, text, uid, Result, Store};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
use walkdir::WalkDir;
fn fingerprint(dir: &Path) -> Result<(String, u64)> {
    let mut entries = WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(err)?;
    entries.sort_by(|a, b| a.path().cmp(b.path()));
    let mut hash = Sha256::new();
    let mut bytes = 0;
    for entry in entries {
        if entry.file_type().is_symlink() {
            return Err("工作区包含符号链接，拒绝删除或移动".into());
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry
            .path()
            .strip_prefix(dir)
            .map_err(err)?
            .to_string_lossy();
        let mut file = fs::File::open(entry.path()).map_err(err)?;
        let len = file.metadata().map_err(err)?.len();
        hash.update((path.len() as u64).to_le_bytes());
        hash.update(path.as_bytes());
        hash.update(len.to_le_bytes());
        let mut buffer = [0u8; 65536];
        loop {
            let count = file.read(&mut buffer).map_err(err)?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        bytes += len;
    }
    Ok((format!("{:x}", hash.finalize()), bytes))
}
impl Store {
    pub(crate) fn remove_workspace_preview(&self, q: &Value) -> Result<Value> {
        let (w, dir) = self.workspace(text(q, "workspaceId"))?;
        let (hash, bytes) = fingerprint(&dir)?;
        Ok(
            json!({"workspaceId":w["id"],"name":w["name"],"remote":w["remote"],"fingerprint":hash,"bytes":bytes,"pageCount":Self::pages_at(&dir)?.len()}),
        )
    }
    pub(crate) fn remove_workspace(&mut self, q: &Value) -> Result<Value> {
        if q["confirmed"] != true {
            return Err("请确认只移除此设备的工作区，远程仓库不会删除".into());
        }
        let current = self.remove_workspace_preview(q)?;
        if current != q["preview"] {
            return Err("工作区内容或设置已有变化，请重新确认移除".into());
        }
        let id = text(q, "workspaceId");
        let (w, dir) = self.workspace(id)?;
        self.clear_credentials(&w)?;
        let archive_id = uid();
        let archive_root = self.root.join("archived-workspaces");
        ensure_dir(&archive_root)?;
        let dest = archive_root.join(&archive_id);
        fs::rename(&dir, &dest).map_err(err)?;
        let before = self.config.clone();
        if !self.config["archivedWorkspaces"].is_array() {
            self.config["archivedWorkspaces"] = json!([]);
        }
        self.config["archivedWorkspaces"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":archive_id,"workspace":w,"removedAt":now()}));
        self.config["workspaces"]
            .as_array_mut()
            .unwrap()
            .retain(|w| text(w, "id") != id);
        let mut fallback = None;
        let result = (|| {
            if self.config["workspaces"].as_array().unwrap().is_empty() {
                fallback = Some(self.create_workspace(&json!({"name":"个人笔记"}))?);
            } else if text(&self.config, "activeId") == id {
                self.config["activeId"] = self.config["workspaces"][0]["id"].clone();
            }
            self.persist()
        })();
        if let Err(e) = result {
            self.config = before;
            let rollback = fs::rename(&dest, &dir);
            if let Some(w) = fallback {
                let _ = fs::remove_dir_all(self.root.join("workspaces").join(text(&w, "id")));
            }
            let _ = self.persist();
            return Err(format!(
                "移除未完成：{e}{}",
                if rollback.is_err() {
                    "；目录仍在已移除目录中，请保留数据目录"
                } else {
                    ""
                }
            ));
        }
        self.credentials.remove(id);
        Ok(self.config.clone())
    }
    pub(crate) fn archived_workspaces(&self) -> Result<Value> {
        let mut list = self
            .config
            .get("archivedWorkspaces")
            .cloned()
            .unwrap_or(json!([]));
        for item in list.as_array_mut().ok_or("已移除工作区列表无效")? {
            let id = text(item, "id");
            if !crate::valid_id(id) {
                return Err("归档标识无效".into());
            }
            let dir = self.root.join("archived-workspaces").join(id);
            let (hash, bytes) = if !dir.exists() && item["purging"] == true {
                (String::new(), 0)
            } else {
                fingerprint(&dir)?
            };
            item["fingerprint"] = json!(hash);
            item["bytes"] = json!(bytes);
        }
        Ok(list)
    }
    pub(crate) fn restore_workspace(&mut self, q: &Value) -> Result<Value> {
        let id = text(q, "archiveId");
        if !crate::valid_id(id) {
            return Err("归档标识无效".into());
        }
        let archive = self.config["archivedWorkspaces"]
            .as_array()
            .and_then(|a| a.iter().find(|a| text(a, "id") == id))
            .cloned()
            .ok_or("已移除工作区不存在")?;
        if archive["purging"] == true {
            return Err("此工作区已开始永久删除，不能恢复；请继续清理剩余空间".into());
        }
        let w = &archive["workspace"];
        let wid = text(w, "id");
        if !crate::valid_id(wid) {
            return Err("工作区标识无效".into());
        }
        if self.config["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| text(w, "id") == wid)
        {
            return Err("同名标识的工作区已经存在".into());
        }
        let source = self.root.join("archived-workspaces").join(id);
        fingerprint(&source)?;
        Self::pages_at(&source)?;
        let dest = self.root.join("workspaces").join(wid);
        if dest.exists() {
            return Err("恢复目标目录已存在，未覆盖".into());
        }
        fs::rename(&source, &dest).map_err(err)?;
        let before = self.config.clone();
        self.config["archivedWorkspaces"]
            .as_array_mut()
            .unwrap()
            .retain(|v| text(v, "id") != id);
        self.config["workspaces"]
            .as_array_mut()
            .unwrap()
            .push(w.clone());
        self.config["activeId"] = w["id"].clone();
        if let Err(e) = self.persist() {
            self.config = before;
            fs::rename(dest, source).map_err(err)?;
            return Err(e);
        }
        Ok(w.clone())
    }
    pub(crate) fn purge_workspace(&mut self, q: &Value) -> Result<Value> {
        if q["confirmed"] != true {
            return Err("永久删除无法撤销，请再次确认".into());
        }
        let id = text(q, "archiveId");
        let list = self.archived_workspaces()?;
        let archive = list
            .as_array()
            .unwrap()
            .iter()
            .find(|v| text(v, "id") == id)
            .ok_or("已移除工作区不存在")?;
        if q["fingerprint"] != archive["fingerprint"] {
            return Err("已移除工作区内容已有变化，请重新确认永久删除".into());
        }
        let before = self.config.clone();
        for entry in self.config["archivedWorkspaces"].as_array_mut().unwrap() {
            if text(entry, "id") == id {
                entry["purging"] = json!(true);
            }
        }
        if let Err(e) = self.persist() {
            self.config = before;
            return Err(e);
        }
        // 先记录删除意图，清理中断时仍能从列表重试；远程仓库不受影响。
        let workspace_id = text(&archive["workspace"], "id");
        if !crate::valid_id(workspace_id) {
            return Err("归档工作区标识无效".into());
        }
        let backups = self.root.join("sync-draft-backups");
        if backups.exists() {
            ensure_dir(&backups)?;
            let drafts = backups.join(workspace_id);
            if drafts.exists() {
                ensure_dir(&drafts)?;
                fs::remove_dir_all(drafts)
                    .map_err(|e| format!("同步草稿恢复点清理未完成，请重试：{e}"))?;
            }
        }
        let dir = self.root.join("archived-workspaces").join(id);
        if dir.exists() {
            fs::remove_dir_all(&dir)
                .map_err(|e| format!("空间清理未完成，部分文件可能已删除，请重试：{e}"))?;
        }
        self.finish_purge(id)?;
        Ok(json!({"bytes":archive["bytes"]}))
    }
    fn finish_purge(&mut self, id: &str) -> Result<()> {
        let pending = self.config.clone();
        self.config["archivedWorkspaces"]
            .as_array_mut()
            .unwrap()
            .retain(|v| text(v, "id") != id);
        if let Err(e) = self.persist() {
            self.config = pending;
            return Err(format!(
                "本机文件已清理，列表更新失败；可重新打开管理列表后继续清理：{e}"
            ));
        }
        Ok(())
    }
    pub(crate) fn restore_sync_backup(&mut self, q: &Value) -> Result<Value> {
        let (w, source) = self.workspace(text(q, "workspaceId"))?;
        let id = text(q, "backupId");
        let side = text(q, "side");
        let backups = crate::sync::backups(&source)?;
        if !backups
            .as_array()
            .map(|a| a.iter().any(|b| text(b, "id") == id))
            .unwrap_or(false)
            || !["local", "remote"].contains(&side)
        {
            return Err("恢复点或版本无效".into());
        }
        let before = self.config.clone();
        let new =
            self.create_workspace(&json!({"name":format!("{} · 同步恢复",text(&w,"name"))}))?;
        let (_, dest) = self.workspace(text(&new, "id"))?;
        if let Err(e) = crate::sync::export_backup(&source, id, side, &dest) {
            self.config = before;
            self.persist()?;
            let _ = fs::remove_dir_all(dest);
            return Err(e);
        }
        Ok(new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn purge_list_persist_failure_keeps_retryable_pending_state() {
        let t = tempfile::tempdir().unwrap();
        let mut s = Store::new(t.path()).unwrap();
        s.config["archivedWorkspaces"] =
            json!([{"id":"archive","workspace":{"id":"old","name":"已删数据"},"purging":true}]);
        s.persist().unwrap();
        fs::remove_file(t.path().join("config.json")).unwrap();
        fs::create_dir(t.path().join("config.json")).unwrap();
        assert!(s.finish_purge("archive").is_err());
        let list = s.archived_workspaces().unwrap();
        assert_eq!(list[0]["purging"], true);
        assert_eq!(list[0]["bytes"], 0);
        fs::remove_dir(t.path().join("config.json")).unwrap();
        s.finish_purge("archive").unwrap();
        assert!(s
            .archived_workspaces()
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty());
    }
}
