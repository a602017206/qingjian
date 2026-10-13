mod cleanup;
mod git;
pub use git::configure_network;
mod import;
mod markdown;
mod notion;

use base64::Engine;
use chrono::Utc;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;

pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub(crate) fn text<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}
pub(crate) fn uid() -> String {
    Uuid::new_v4().simple().to_string()
}
pub(crate) fn now() -> String {
    Utc::now().to_rfc3339()
}
pub(crate) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
pub(crate) fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(err)?;
    if fs::symlink_metadata(path)
        .map_err(err)?
        .file_type()
        .is_symlink()
    {
        return Err("数据目录不支持符号链接".into());
    }
    Ok(())
}
pub(crate) fn atomic(path: &Path, data: &[u8]) -> Result<()> {
    let temp = path.with_file_name(format!(".pending-{}", uid()));
    let result = (|| {
        let mut f = File::create(&temp).map_err(err)?;
        f.write_all(data).map_err(err)?;
        f.sync_all().map_err(err)?;
        fs::rename(&temp, path).map_err(err)
    })();
    let _ = fs::remove_file(temp);
    result
}
pub(crate) fn write_json(path: &Path, value: &Value) -> Result<()> {
    let mut b = serde_json::to_vec_pretty(value).map_err(err)?;
    b.push(b'\n');
    if b.len() > 16 << 20 {
        return Err("页面实际文件超过 16 兆字节，请拆分内容；已有文件未覆盖".into());
    }
    atomic(path, &b)
}

pub(crate) fn validate_page(page: &Value) -> Result<()> {
    if !page.is_object() {
        return Err("笔记必须是页面对象".into());
    }
    if let Some(blocks) = page.get("blocks") {
        let blocks = blocks.as_array().ok_or("页面内容块格式无效")?;
        let mut ids = HashSet::new();
        for block in blocks {
            if !block.is_object()
                || !valid_id(text(block, "id"))
                || !block["text"].is_string()
                || text(block, "type").is_empty()
                || !ids.insert(text(block, "id"))
            {
                return Err("内容块标识、类型或正文格式无效".into());
            }
        }
    }
    if let Some(tags) = page.get("tags") {
        if !tags
            .as_array()
            .map(|tags| tags.iter().all(Value::is_string))
            .unwrap_or(false)
        {
            return Err("页面标签格式无效".into());
        }
    }
    if let Some(db) = page.get("database").filter(|db| !db.is_null()) {
        let columns = db["columns"].as_array().ok_or("数据库属性格式无效")?;
        let rows = db["rows"].as_array().ok_or("数据库记录格式无效")?;
        if columns.is_empty()
            || columns.iter().any(|c| {
                !c.is_object()
                    || !valid_id(text(c, "id"))
                    || !c["name"].is_string()
                    || !c["type"].is_string()
            })
        {
            return Err("数据库属性格式无效".into());
        }
        let mut column_ids = HashSet::new();
        for column in columns {
            if !column_ids.insert(text(column, "id")) {
                return Err("数据库属性标识重复".into());
            }
            if let Some(options) = column.get("options") {
                if !options
                    .as_array()
                    .map(|items| items.iter().all(Value::is_string))
                    .unwrap_or(false)
                {
                    return Err("属性选项格式无效".into());
                }
            }
            if let Some(formula) = column.get("formula") {
                if !formula.is_string() {
                    return Err("公式必须是文本表达式".into());
                }
            }
            for key in ["targetDatabaseId", "relationId", "targetPropertyId"] {
                if let Some(value) = column.get(key) {
                    if !value.is_string()
                        || (!text(column, key).is_empty() && !valid_id(text(column, key)))
                    {
                        return Err("属性关联标识无效".into());
                    }
                }
            }
        }
        let mut row_ids = HashSet::new();
        for row in rows {
            if !row_ids.insert(text(row, "id")) {
                return Err("数据库记录标识重复".into());
            }
        }
        if rows.iter().any(|r| {
            !r.is_object()
                || !valid_id(text(r, "id"))
                || !r["values"]
                    .as_object()
                    .map(|values| values.values().all(Value::is_string))
                    .unwrap_or(false)
        }) {
            return Err("数据库记录值格式无效".into());
        }
    }
    Ok(())
}

pub(crate) fn validate_hierarchy(pages: &[Value]) -> Result<()> {
    let by_id: std::collections::HashMap<_, _> = pages.iter().map(|p| (text(p, "id"), p)).collect();
    if by_id.len() != pages.len() {
        return Err("页面标识重复".into());
    }
    for p in pages {
        let mut current = text(p, "id");
        let mut seen = HashSet::new();
        while !current.is_empty() {
            if !seen.insert(current) {
                return Err("导入或远程页面的层级形成循环，未应用修改".into());
            }
            let page = by_id.get(current).ok_or("页面引用了不存在的父页面")?;
            current = text(page, "parentId");
        }
    }
    Ok(())
}
pub(crate) fn read_json(path: &Path) -> Result<Value> {
    let meta = fs::symlink_metadata(path).map_err(err)?;
    if meta.file_type().is_symlink() {
        return Err("数据文件不支持符号链接".into());
    }
    if meta.len() > 16 << 20 {
        return Err("笔记文件过大，请拆分内容".into());
    }
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(|e| {
        format!(
            "数据文件 {} 损坏：{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            e
        )
    })
}

pub struct Store {
    pub root: PathBuf,
    pub(crate) config: Value,
    pub(crate) credentials: std::collections::HashMap<String, (String, String)>,
}
impl Store {
    pub fn new(root: &Path) -> Result<Self> {
        ensure_dir(root)?;
        ensure_dir(&root.join("workspaces"))?;
        let config_path = root.join("config.json");
        let config = if config_path.exists() {
            read_json(&config_path)?
        } else {
            json!({"workspaces":[],"activeId":"","preferences":{"language":"zh-CN","theme":"light","font":"sans"}})
        };
        if !config["workspaces"].is_array() {
            return Err("配置中的工作区列表损坏".into());
        };
        Ok(Self {
            root: root.to_owned(),
            config,
            credentials: Default::default(),
        })
    }
    fn persist(&self) -> Result<()> {
        write_json(&self.root.join("config.json"), &self.config)
    }
    pub(crate) fn workspace(&self, id: &str) -> Result<(Value, PathBuf)> {
        if !valid_id(id) {
            return Err("工作区标识无效".into());
        };
        let w = self.config["workspaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| text(w, "id") == id)
            .cloned()
            .ok_or("工作区不存在")?;
        let dir = self.root.join("workspaces").join(id);
        ensure_dir(&dir)?;
        for child in ["pages", "assets"] {
            ensure_dir(&dir.join(child))?;
        }
        Ok((w, dir))
    }
    pub(crate) fn pages_at(dir: &Path) -> Result<Vec<Value>> {
        ensure_dir(&dir.join("pages"))?;
        let mut pages = vec![];
        for entry in fs::read_dir(dir.join("pages")).map_err(err)? {
            let entry = entry.map_err(err)?;
            if entry.path().extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            };
            let mut p = read_json(&entry.path())?;
            validate_page(&p)?;
            if !valid_id(text(&p, "id"))
                || entry.file_name() != format!("{}.json", text(&p, "id")).as_str()
            {
                return Err("笔记标识与文件名不一致".into());
            };
            if !p["blocks"].is_array() {
                p["blocks"] = json!([])
            };
            if !p["tags"].is_array() {
                p["tags"] = json!([])
            };
            pages.push(p)
        }
        pages.sort_by(|a, b| text(a, "createdAt").cmp(text(b, "createdAt")));
        validate_hierarchy(&pages)?;
        Ok(pages)
    }
    pub(crate) fn save_at(dir: &Path, mut p: Value) -> Result<Value> {
        validate_page(&p)?;
        if !p.is_object() {
            return Err("页面格式无效".into());
        };
        let pages = Self::pages_at(dir)?;
        let id = if text(&p, "id").is_empty() {
            uid()
        } else {
            text(&p, "id").to_owned()
        };
        if !valid_id(&id) {
            return Err("页面标识无效".into());
        };
        let old = pages.iter().find(|o| text(o, "id") == id);
        if let Some(old) = old {
            if p["revision"] != old["revision"] {
                return Err("页面已在其他窗口更新，请刷新后继续。当前编辑没有覆盖磁盘内容".into());
            };
            p["createdAt"] = old["createdAt"].clone();
            p["revision"] = json!(old["revision"].as_u64().unwrap_or(0) + 1)
        } else {
            p["createdAt"] = json!(now());
            p["revision"] = json!(1)
        }
        let mut parent = text(&p, "parentId");
        let mut seen = HashSet::from([id.clone()]);
        while !parent.is_empty() {
            if !seen.insert(parent.into()) {
                return Err("页面层级不能形成循环".into());
            };
            let ancestor = pages
                .iter()
                .find(|x| text(x, "id") == parent)
                .ok_or("父页面不存在")?;
            if ancestor["deleted"] == true {
                return Err("父页面已删除，请先恢复父页面".into());
            };
            parent = text(ancestor, "parentId")
        }
        p["id"] = json!(id);
        p["updatedAt"] = json!(now());
        if text(&p, "title").trim().is_empty() {
            p["title"] = json!("未命名页面")
        };
        if text(&p, "icon").is_empty() {
            p["icon"] = json!("📄")
        };
        if !p["tags"].is_array() {
            p["tags"] = json!([])
        };
        if !p["blocks"].is_array() {
            p["blocks"] = json!([])
        };
        for key in ["favorite", "deleted", "wide", "locked"] {
            if !p[key].is_boolean() {
                p[key] = json!(false)
            }
        }
        let bytes = serde_json::to_vec(&p).map_err(err)?;
        if bytes.len() > 16 << 20 {
            return Err("页面内容超过 16 兆字节，请拆分页面".into());
        };
        write_json(
            &dir.join("pages").join(format!("{}.json", text(&p, "id"))),
            &p,
        )?;
        Ok(p)
    }
    fn create_workspace(&mut self, q: &Value) -> Result<Value> {
        let name = text(q, "name").trim();
        if name.is_empty() {
            return Err("请输入工作区名称".into());
        };
        let remote = text(q, "remote");
        git::validate_remote(remote)?;
        let branch = if text(q, "branch").is_empty() {
            "main"
        } else {
            text(q, "branch")
        };
        if !git2::Reference::is_valid_name(&format!("refs/heads/{branch}")) {
            return Err("分支名称无效".into());
        };
        let id = uid();
        let dir = self.root.join("workspaces").join(&id);
        let outcome = (|| {
            if text(q, "mode") == "clone" {
                if remote.is_empty() {
                    return Err("克隆需要远程地址".into());
                };
                git::clone_repo(remote, branch, &dir, text(q, "username"), text(q, "token"))?;
            } else {
                ensure_dir(&dir)?;
                let mut options = git2::RepositoryInitOptions::new();
                options.initial_head(branch);
                git2::Repository::init_opts(&dir, &options).map_err(err)?;
            }
            ensure_dir(&dir.join("pages"))?;
            ensure_dir(&dir.join("assets"))?;
            Self::pages_at(&dir)?;
            let repo = git2::Repository::open(&dir).map_err(err)?;
            if !remote.is_empty() && text(q, "mode") != "clone" {
                repo.remote("origin", remote).map_err(err)?;
            }
            if !dir.join("workspace.json").exists() {
                write_json(
                    &dir.join("workspace.json"),
                    &json!({"formatVersion":1,"name":name}),
                )?
            };
            Ok(())
        })();
        if let Err(e) = outcome {
            let _ = fs::remove_dir_all(&dir);
            return Err(e);
        }
        let w = json!({"id":id,"name":name,"remote":remote,"branch":branch,"createdAt":now()});
        let original = self.config.clone();
        self.config["workspaces"]
            .as_array_mut()
            .unwrap()
            .push(w.clone());
        self.config["activeId"] = json!(id);
        if let Err(e) = self.persist() {
            self.config = original;
            return Err(e);
        };
        if !text(q, "token").is_empty() {
            self.credentials
                .insert(id, (text(q, "username").into(), text(q, "token").into()));
        };
        Ok(w)
    }
    fn trash(&self, dir: &Path, id: &str, deleted: bool) -> Result<Value> {
        let pages = Self::pages_at(dir)?;
        if !pages.iter().any(|p| text(p, "id") == id) {
            return Err("页面不存在".into());
        };
        let mut ids = HashSet::from([id.to_owned()]);
        loop {
            let before = ids.len();
            for p in &pages {
                if ids.contains(text(p, "parentId")) {
                    ids.insert(text(p, "id").into());
                }
            }
            if before == ids.len() {
                break;
            }
        }
        if !deleted {
            let mut parent = pages
                .iter()
                .find(|p| text(p, "id") == id)
                .map(|p| text(p, "parentId"))
                .unwrap_or("");
            while !parent.is_empty() {
                ids.insert(parent.into());
                parent = pages
                    .iter()
                    .find(|p| text(p, "id") == parent)
                    .map(|p| text(p, "parentId"))
                    .unwrap_or("");
            }
        }
        for mut p in pages {
            if ids.contains(text(&p, "id")) {
                p["deleted"] = json!(deleted);
                p["updatedAt"] = json!(now());
                p["revision"] = json!(p["revision"].as_u64().unwrap_or(0) + 1);
                write_json(
                    &dir.join("pages").join(format!("{}.json", text(&p, "id"))),
                    &p,
                )?;
            }
        }
        Ok(json!(true))
    }
    fn stats(&self, dir: &Path) -> Result<Value> {
        let mut notes = 0u64;
        let mut assets = 0u64;
        let mut git = 0u64;
        for e in WalkDir::new(dir).follow_links(false) {
            let e = e.map_err(err)?;
            if e.file_type().is_symlink() {
                return Err("工作区包含符号链接，请移除后重试".into());
            };
            if !e.file_type().is_file() {
                continue;
            };
            let len = e.metadata().map_err(err)?.len();
            let rel = e.path().strip_prefix(dir).map_err(err)?;
            if rel.starts_with(".git") {
                git += len
            } else if rel.starts_with("assets") {
                assets += len
            } else {
                notes += len
            }
        }
        Ok(
            json!({"notes":notes,"assets":assets,"git":git,"total":notes+assets+git,"pageCount":Self::pages_at(dir)?.len(),"path":dir}),
        )
    }
    pub fn dispatch(&mut self, action: &str, q: Value) -> Result<Value> {
        match action {
            "bootstrap" => {
                if self.config["workspaces"].as_array().unwrap().is_empty() {
                    let w = self.create_workspace(&json!({"name":"个人笔记"}))?;
                    let (_, dir) = self.workspace(text(&w, "id"))?;
                    for page in seed_pages() {
                        Self::save_at(&dir, page)?;
                    }
                }
                Ok(self.config.clone())
            }
            "createWorkspace" => self.create_workspace(&q),
            "activate" => {
                self.workspace(text(&q, "workspaceId"))?;
                self.config["activeId"] = q["workspaceId"].clone();
                self.persist()?;
                Ok(self.config.clone())
            }
            "preferences" => {
                let p = &q["preferences"];
                if !["zh-CN", "zh-TW"].contains(&text(p, "language"))
                    || !["light", "dark", "system"].contains(&text(p, "theme"))
                {
                    return Err("语言或主题设置无效".into());
                };
                self.config["preferences"] = p.clone();
                self.persist()?;
                Ok(self.config.clone())
            }
            "updateWorkspace" => {
                let new = &q["workspace"];
                let (old, dir) = self.workspace(text(new, "id"))?;
                if text(new, "name").trim().is_empty() {
                    return Err("名称不能为空".into());
                };
                if text(new, "branch") != text(&old, "branch") {
                    return Err("已有工作区暂不支持切换分支，请新建工作区克隆目标分支".into());
                };
                git::validate_remote(text(new, "remote"))?;
                let repo = git2::Repository::open(dir).map_err(err)?;
                if text(new, "remote").is_empty() {
                    if repo.find_remote("origin").is_ok() {
                        repo.remote_delete("origin").map_err(err)?;
                    }
                } else if repo.find_remote("origin").is_ok() {
                    repo.remote_set_url("origin", text(new, "remote"))
                        .map_err(err)?;
                } else {
                    repo.remote("origin", text(new, "remote")).map_err(err)?;
                };
                for w in self.config["workspaces"].as_array_mut().unwrap() {
                    if w["id"] == new["id"] {
                        w["name"] = new["name"].clone();
                        w["remote"] = new["remote"].clone();
                    }
                }
                self.persist()?;
                Ok(self.config.clone())
            }
            "credentials" => {
                let id = text(&q, "workspaceId");
                self.workspace(id)?;
                self.credentials.insert(
                    id.into(),
                    (text(&q, "username").into(), text(&q, "token").into()),
                );
                Ok(json!(true))
            }
            _ => {
                let (w, dir) = self.workspace(text(&q, "workspaceId"))?;
                match action {
                    "pages" => Ok(json!(Self::pages_at(&dir)?)),
                    "save" => Self::save_at(&dir, q["page"].clone()),
                    "trash" => self.trash(&dir, text(&q, "id"), q["deleted"] == true),
                    "trashMany" => cleanup::trash_many(&dir, &q),
                    "purgePreview" => cleanup::preview(&dir, &q),
                    "purgeTrash" => cleanup::purge(&dir, &q),
                    "stats" => self.stats(&dir),
                    "status" => git::status(&dir, &w),
                    "commit" => git::commit(&dir, text(&q, "message")),
                    "sync" => {
                        let auth = self
                            .credentials
                            .get(text(&w, "id"))
                            .cloned()
                            .unwrap_or_default();
                        git::sync(&dir, &w, &auth.0, &auth.1)
                    }
                    "history" => git::history(&dir, text(&q, "id")),
                    "restore" => {
                        git::commit(&dir, "恢复前保存笔记")?;
                        let mut p = git::read_version(&dir, text(&q, "id"), text(&q, "commit"))?;
                        let current = Self::pages_at(&dir)?
                            .into_iter()
                            .find(|p| text(p, "id") == text(&q, "id"));
                        if let Some(current) = current {
                            p["revision"] = current["revision"].clone()
                        }
                        p["deleted"] = json!(false);
                        git::restore_page_assets(&dir, &p, text(&q, "commit"))?;
                        Self::save_at(&dir, p)
                    }
                    "import" => {
                        let b = base64::engine::general_purpose::STANDARD
                            .decode(text(&q, "data"))
                            .map_err(|_| "文件编码无效")?;
                        import::import(&dir, text(&q, "name"), &b)
                    }
                    "export" => import::export(&dir),
                    "asset" => {
                        let name = text(&q, "name");
                        if name.contains(['/', '\\'])
                            || name == "."
                            || name == ".."
                            || name.is_empty()
                        {
                            return Err("附件路径无效".into());
                        };
                        let path = dir.join("assets").join(name);
                        if fs::symlink_metadata(&path)
                            .map_err(err)?
                            .file_type()
                            .is_symlink()
                        {
                            return Err("附件不支持符号链接".into());
                        };
                        let b = fs::read(path).map_err(err)?;
                        Ok(
                            json!({"data":base64::engine::general_purpose::STANDARD.encode(b),"name":name}),
                        )
                    }
                    "attach" => {
                        let b = base64::engine::general_purpose::STANDARD
                            .decode(text(&q, "data"))
                            .map_err(err)?;
                        if b.len() > 32 << 20 {
                            return Err("附件超过 32 兆字节，请通过外部链接引用".into());
                        };
                        let name = import::asset_name(text(&q, "name"), &b);
                        let path = dir.join("assets").join(&name);
                        if !path.exists() {
                            atomic(&path, &b)?
                        };
                        Ok(json!({"name":name}))
                    }
                    _ => Err("此操作不存在".into()),
                }
            }
        }
    }
}
fn seed_pages() -> Vec<Value> {
    let blocks=markdown::parse("## 从一个想法开始\n\n像熟悉的笔记工具一样写作，但所有内容都属于你。页面、数据库和附件保存在本地文件中，随时可以导出。\n\n- [ ] 新建你的第一篇笔记\n- [ ] 在设置中关联一个远程仓库\n- [ ] 导入已有的 Notion 笔记\n\n## 让写作更顺手\n\n- 输入 / 打开块菜单，选择标题、待办、引用或代码。\n- 拖动左侧手柄调整顺序；使用 [[页面名称]] 关联其他页面。\n- 按 Ctrl / ⌘ + K 搜索，按 Ctrl / ⌘ + S 立即保存。\n\n## 你的笔记，你的版本\n\n编辑会自动保存到磁盘。点击「保存版本」创建 Git 提交；连接远程后，点击「同步」在设备之间传递笔记。");
    let mut intro = vec![
        json!({"id":uid(),"type":"callout","text":"这里是你的本地知识空间。无需账号，断网也能写作。"}),
    ];
    intro.extend(blocks);
    vec![
        json!({"title":"开始使用轻笺","icon":"🌿","favorite":true,"cover":"garden","tags":["指南"],"blocks":intro}),
        json!({"title":"产品设计手记","icon":"✏️","favorite":true,"tags":["设计","灵感"],"blocks":markdown::parse("## 好的工具，让注意力回到内容\n\n减少界面的噪声，留出足够的书写空间。功能应该在需要时出现，而不是始终占据视野。\n\n> 先理解问题，再设计答案。")}),
        json!({"title":"阅读清单","icon":"📚","tags":["阅读"],"blocks":markdown::parse("## 想读的书\n\n- [ ] 设计心理学\n- [ ] 认知觉醒\n\n## 阅读笔记\n\n把最有触动的一句话留在这里。")}),
        json!({"title":"项目看板","icon":"🗂️","tags":["计划"],"database":{"columns":[{"id":"name","name":"任务名称","type":"text"},{"id":"status","name":"状态","type":"select","options":["待开始","进行中","已完成"]},{"id":"priority","name":"优先级","type":"select","options":["高","中","低"]},{"id":"date","name":"日期","type":"date"}],"rows":[{"id":uid(),"values":{"name":"整理想法与需求","status":"已完成","priority":"高"}},{"id":uid(),"values":{"name":"搭建个人知识库","status":"进行中","priority":"高"}},{"id":uid(),"values":{"name":"整理阅读笔记","status":"待开始","priority":"中"}}]},"blocks":[]}),
        json!({"title":"随手记","icon":"💡","tags":["灵感"],"blocks":markdown::parse("想到什么，就记下来。暂时不用分类，也不必写得完整。")}),
    ]
}
