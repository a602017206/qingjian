use crate::{
    atomic, ensure_dir, err, git, json, now, read_json, text, uid, valid_id, validate_page,
    write_json, Result, Store, Value,
};
impl Store {
    pub(crate) fn auto_sync_settings(&mut self, q: &Value) -> Result<Value> {
        let id = text(q, "workspaceId");
        self.workspace(id)?;
        let enabled = q["enabled"].as_bool().ok_or("自动同步设置必须是开关值")?;
        let before = self.config.clone();
        for w in self.config["workspaces"].as_array_mut().unwrap() {
            if text(w, "id") == id {
                w["autoSync"] = json!(enabled);
            }
        }
        if let Err(error) = self.persist() {
            self.config = before;
            return Err(error);
        }
        Ok(self.config.clone())
    }
    fn sync_draft_path(&self, workspace_id: &str, backup_id: &str) -> Result<std::path::PathBuf> {
        self.workspace(workspace_id)?;
        if !valid_id(backup_id) {
            return Err("同步草稿恢复点标识无效".into());
        }
        let root = self.root.join("sync-draft-backups");
        ensure_dir(&root)?;
        let dir = root.join(workspace_id);
        ensure_dir(&dir)?;
        Ok(dir.join(format!("{backup_id}.json")))
    }
    pub(crate) fn sync_draft_preview(&self, q: &Value) -> Result<Value> {
        let id = text(q, "workspaceId");
        let (_, dir) = self.workspace(id)?;
        let draft = &q["draft"];
        validate_page(draft)?;
        if !valid_id(text(draft, "id")) {
            return Err("同步草稿页面标识无效".into());
        }
        let remote = Self::pages_at(&dir)?
            .into_iter()
            .find(|p| p["id"] == draft["id"]);
        let backup_id = uid();
        let path = self.sync_draft_path(id, &backup_id)?;
        ensure_dir(&dir.join("assets"))?;
        let assets = path.with_extension("assets");
        ensure_dir(&assets)?;
        let mut names = vec![];
        let source_tree = if text(q, "sourceCommit").is_empty() {
            None
        } else {
            let repo = git2::Repository::open(&dir).map_err(err)?;
            let oid = git2::Oid::from_str(text(q, "sourceCommit")).map_err(err)?;
            let commit = repo.find_commit(oid).map_err(err)?;
            Some((repo.path().to_path_buf(), commit.tree_id()))
        };
        for name in crate::cleanup::asset_refs(draft) {
            let historical = if let Some((repo_path, tree_id)) = &source_tree {
                let repo = git2::Repository::open(repo_path).map_err(err)?;
                let tree = repo.find_tree(*tree_id).map_err(err)?;
                match tree.get_path(&std::path::Path::new("assets").join(&name)) {
                    Ok(entry) => {
                        if ![0o100644, 0o100755].contains(&entry.filemode()) {
                            return Err("草稿历史附件不是普通文件".into());
                        }
                        let blob = repo.find_blob(entry.id()).map_err(err)?;
                        if blob.size() > 128 << 20 {
                            return Err("草稿历史附件超过恢复限制".into());
                        }
                        Some(blob.content().to_vec())
                    }
                    Err(error) if error.code() == git2::ErrorCode::NotFound => None,
                    Err(error) => return Err(err(error)),
                }
            } else {
                None
            };
            let bytes = if let Some(bytes) = historical {
                bytes
            } else {
                let live = dir.join("assets").join(&name);
                let meta = std::fs::symlink_metadata(&live)
                    .map_err(|_| format!("草稿附件缺失：{name}；请保留原设备的数据"))?;
                if !meta.file_type().is_file() || meta.len() > 128 << 20 {
                    return Err("草稿附件不是普通文件或超过恢复限制".into());
                }
                std::fs::read(&live).map_err(err)?
            };
            atomic(&assets.join(&name), &bytes)?;
            names.push(name);
        }
        let preview = json!({"id":backup_id,"workspaceId":id,"createdAt":now(),"draft":draft,"remote":remote,"assets":names});
        write_json(&path, &preview)?;
        Ok(preview)
    }
    pub(crate) fn resolve_sync_draft(&self, q: &Value, restore: bool) -> Result<Value> {
        let id = text(q, "workspaceId");
        let (_, dir) = self.workspace(id)?;
        let preview = read_json(&self.sync_draft_path(id, text(q, "backupId"))?)?;
        if text(&preview, "workspaceId") != id {
            return Err("同步草稿恢复点不属于当前工作区".into());
        }
        let choice = if restore { "copy" } else { text(q, "choice") };
        if !["local", "remote", "copy"].contains(&choice) {
            return Err("请选择保留本机、已同步版本或另存副本".into());
        }
        let remote = Self::pages_at(&dir)?
            .into_iter()
            .find(|p| p["id"] == preview["draft"]["id"]);
        if choice != "copy" && json!(remote) != preview["remote"] {
            return Err("已同步页面又有变化，请重新核对后选择，草稿恢复点仍保留".into());
        }
        if choice == "remote" {
            return Ok(json!({"page":remote}));
        }
        let mut page = preview["draft"].clone();
        git::commit(&dir, "选择同步草稿前保存已同步版本")?;
        let assets = self
            .sync_draft_path(id, text(q, "backupId"))?
            .with_extension("assets");
        ensure_dir(&assets)?;
        ensure_dir(&dir.join("assets"))?;
        for name in crate::cleanup::asset_refs(&page) {
            let source = assets.join(&name);
            let meta = std::fs::symlink_metadata(&source).map_err(err)?;
            if !meta.file_type().is_file() || meta.len() > 128 << 20 {
                return Err("草稿备份附件无效".into());
            }
            let bytes = std::fs::read(source).map_err(err)?;
            let dest = dir.join("assets").join(&name);
            match std::fs::symlink_metadata(&dest) {
                Ok(meta) if meta.file_type().is_file() => {
                    if std::fs::read(&dest).map_err(err)? != bytes {
                        return Err(format!("附件同名但内容不同，拒绝覆盖：{name}"));
                    }
                }
                Ok(_) => return Err("附件目标路径不是普通文件".into()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    atomic(&dest, &bytes)?
                }
                Err(error) => return Err(err(error)),
            }
        }
        if choice == "copy" || remote.is_none() {
            page["id"] = json!("");
            page["revision"] = json!(0);
            page["parentId"] = json!("");
            page["title"] = json!(format!("{} · 同步草稿副本", text(&page, "title")));
            if let Some(rows) = page
                .get_mut("database")
                .and_then(|db| db.get_mut("rows"))
                .and_then(Value::as_array_mut)
            {
                for row in rows {
                    row.as_object_mut()
                        .ok_or("数据库记录格式无效")?
                        .remove("pageId");
                }
            }
        } else if let Some(remote) = &remote {
            page["revision"] = remote["revision"].clone();
        }
        page["deleted"] = json!(false);
        let saved = Self::save_at(&dir, page)?;
        Ok(if restore {
            saved
        } else {
            json!({"page":saved})
        })
    }
    pub(crate) fn sync_draft_backups(&self, q: &Value) -> Result<Value> {
        let id = text(q, "workspaceId");
        self.workspace(id)?;
        let dir = self.root.join("sync-draft-backups").join(id);
        if !dir.exists() {
            return Ok(json!([]));
        }
        ensure_dir(&self.root.join("sync-draft-backups"))?;
        ensure_dir(&dir)?;
        let mut result = vec![];
        for entry in std::fs::read_dir(dir).map_err(err)?.take(1000) {
            let path = entry.map_err(err)?.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let p = read_json(&path)?;
            result
                .push(json!({"id":p["id"],"title":p["draft"]["title"],"createdAt":p["createdAt"]}));
        }
        result.sort_by(|a, b| text(b, "createdAt").cmp(text(a, "createdAt")));
        Ok(json!(result))
    }
}
