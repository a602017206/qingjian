use notebook_core::Store;
use serde_json::json;
fn create(s: &mut Store, name: &str) -> String {
    s.dispatch("createWorkspace", json!({"name":name})).unwrap()["id"]
        .as_str()
        .unwrap()
        .into()
}
#[test]
fn persistence_isolation_and_conflict() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let a = create(&mut s, "个人笔记");
    let b = create(&mut s, "项目");
    let p=s.dispatch("save",json!({"workspaceId":a,"page":{"title":"中文","blocks":[{"id":"b","type":"paragraph","text":"正文"}]}})).unwrap();
    let mut reopened = Store::new(d.path()).unwrap();
    let pages = reopened
        .dispatch("pages", json!({"workspaceId":a}))
        .unwrap();
    assert_eq!(pages[0]["blocks"][0]["text"], "正文");
    assert_eq!(
        reopened
            .dispatch("pages", json!({"workspaceId":b}))
            .unwrap(),
        json!([])
    );
    reopened
        .dispatch("save", json!({"workspaceId":a,"page":p}))
        .unwrap();
    assert!(reopened
        .dispatch("save", json!({"workspaceId":a,"page":p}))
        .is_err());
}
#[test]
fn hierarchy_trash_and_invalid_paths() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "层级");
    let mut parent = s
        .dispatch("save", json!({"workspaceId":w,"page":{"title":"父"}}))
        .unwrap();
    let child = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"子","parentId":parent["id"]}}),
        )
        .unwrap();
    parent["parentId"] = child["id"].clone();
    assert!(s
        .dispatch("save", json!({"workspaceId":w,"page":parent}))
        .is_err());
    s.dispatch(
        "trash",
        json!({"workspaceId":w,"id":parent["id"],"deleted":true}),
    )
    .unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert!(pages
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["deleted"] == true));
    assert!(s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"id":"../../escape"}})
        )
        .is_err());
    assert!(s
        .dispatch(
            "createWorkspace",
            json!({"name":"认证","remote":"https://token@github.com/a/b.git"})
        )
        .is_err());
}
#[test]
fn local_git_history_restore_and_remote_divergence() {
    let d = tempfile::tempdir().unwrap();
    let remote = tempfile::tempdir().unwrap();
    git2::Repository::init_bare(remote.path()).unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let a = s
        .dispatch(
            "createWorkspace",
            json!({"name":"发送","remote":remote.path().to_str().unwrap()}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut p = s
        .dispatch("save", json!({"workspaceId":a,"page":{"title":"原始"}}))
        .unwrap();
    let first = s.dispatch("sync", json!({"workspaceId":a})).unwrap();
    let b = s
        .dispatch(
            "createWorkspace",
            json!({"name":"接收","remote":remote.path().to_str().unwrap(),"mode":"clone"}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut local = s.dispatch("pages", json!({"workspaceId":b})).unwrap()[0].clone();
    p["title"] = json!("远程修改");
    s.dispatch("save", json!({"workspaceId":a,"page":p}))
        .unwrap();
    s.dispatch("sync", json!({"workspaceId":a})).unwrap();
    local["title"] = json!("本地修改");
    s.dispatch("save", json!({"workspaceId":b,"page":local}))
        .unwrap();
    assert!(s
        .dispatch("sync", json!({"workspaceId":b}))
        .unwrap_err()
        .contains("分叉"));
    let history = s
        .dispatch("history", json!({"workspaceId":a,"id":p["id"]}))
        .unwrap();
    assert_eq!(history.as_array().unwrap().len(), 2);
    let restored = s
        .dispatch(
            "restore",
            json!({"workspaceId":a,"id":p["id"],"commit":first["commit"]}),
        )
        .unwrap();
    assert_eq!(restored["title"], "原始");
}
fn zip_files(files: &[(&str, &str)]) -> Vec<u8> {
    use std::io::Write;
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, text) in files {
        z.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(text.as_bytes()).unwrap();
    }
    z.finish().unwrap().into_inner()
}
#[test]
fn notion_import_and_security() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "迁移");
    let data = zip_files(&[
        (
            "旅行.md",
            "# 旅行\n\n[清单](旅行/清单.md)\n\n![图](旅行/图.png)",
        ),
        ("旅行/清单.md", "# 清单\n\n- [x] 相机"),
        ("旅行/图.png", "image"),
        ("任务.csv", "名称,状态\n测试,进行中\n"),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"notion.zip","data":base64::engine::general_purpose::STANDARD.encode(data)})).unwrap();
    assert_eq!(report["pages"], 3);
    assert_eq!(report["assets"], 1);
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    let parent = ps.iter().find(|p| p["title"] == "旅行").unwrap();
    let child = ps.iter().find(|p| p["title"] == "清单").unwrap();
    assert_eq!(child["parentId"], parent["id"]);
    assert_eq!(child["blocks"][0]["checked"], true);
    assert!(parent["blocks"].to_string().contains("page:"));
    let unsafe_zip = zip_files(&[("../escape.md", "# 越界")]);
    assert!(s.dispatch("import",json!({"workspaceId":w,"name":"bad.zip","data":base64::engine::general_purpose::STANDARD.encode(unsafe_zip)})).is_err());
}
#[test]
fn backup_roundtrip_without_duplicates() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let a = create(&mut s, "源");
    let p=s.dispatch("save",json!({"workspaceId":a,"page":{"title":"原文","favorite":true,"tags":["中文"],"blocks":[{"id":"b","type":"callout","text":"提示"}]}})).unwrap();
    let exported = s.dispatch("export", json!({"workspaceId":a})).unwrap();
    let b = create(&mut s, "目标");
    let report = s
        .dispatch(
            "import",
            json!({"workspaceId":b,"name":exported["name"],"data":exported["data"]}),
        )
        .unwrap();
    assert_eq!(report["pages"], 1);
    let pages = s.dispatch("pages", json!({"workspaceId":b})).unwrap();
    assert_eq!(pages[0]["blocks"], p["blocks"]);
    assert_eq!(pages[0]["favorite"], true);
}

#[test]
fn network_configuration_uses_platform_certificate_backend() {
    let dir = tempfile::tempdir().unwrap();
    notebook_core::configure_network(dir.path()).unwrap();
}

#[test]
fn text_attachments_survive_backup_roundtrip() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let a = create(&mut s, "附件源");
    let mut names = vec![];
    for (name, body) in [
        ("说明.md", "# 附件正文"),
        ("数据.csv", "名称,值\n中文,1"),
        ("网页.html", "<h1>附件</h1>"),
        ("配置.json", "{\"key\":\"value\"}"),
    ] {
        let asset=s.dispatch("attach",json!({"workspaceId":a,"name":name,"data":base64::engine::general_purpose::STANDARD.encode(body)})).unwrap();
        names.push((asset["name"].as_str().unwrap().to_owned(), body.to_owned()));
    }
    s.dispatch("save",json!({"workspaceId":a,"page":{"title":"附件页面","blocks":[{"id":"b","type":"paragraph","text":format!("[附件](asset:{})",names[0].0)}]}})).unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":a})).unwrap();
    let b = create(&mut s, "附件目标");
    let report = s
        .dispatch(
            "import",
            json!({"workspaceId":b,"name":backup["name"],"data":backup["data"]}),
        )
        .unwrap();
    assert_eq!(report["pages"], 1);
    assert_eq!(report["assets"], 4);
    for (name, body) in names {
        let asset = s
            .dispatch("asset", json!({"workspaceId":b,"name":name}))
            .unwrap();
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(asset["data"].as_str().unwrap())
                .unwrap(),
            body.as_bytes()
        );
    }
}
#[test]
fn malformed_backup_does_not_panic_or_break_workspace() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "错误文件");
    for payload in [
        "[]",
        "1",
        "{\"id\":\"bad\",\"title\":\"错误\",\"blocks\":[1]}",
    ] {
        let bytes = zip_files(&[
            ("manifest.json", "{\"format\":\"qingjian\",\"version\":1}"),
            ("pages/bad.json", payload),
        ]);
        assert!(s.dispatch("import",json!({"workspaceId":w,"name":"backup.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).is_err());
        assert!(s.dispatch("pages", json!({"workspaceId":w})).is_ok());
    }
    s.dispatch(
        "save",
        json!({"workspaceId":w,"page":{"title":"仍可正常保存"}}),
    )
    .unwrap();
}
#[test]
fn imports_zip_without_filename_extension() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "安卓文件");
    let bytes = zip_files(&[("中文.md", "# 中文\n\n正文")]);
    let result=s.dispatch("import",json!({"workspaceId":w,"name":"123","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    assert_eq!(result["pages"], 1);
}
#[test]
fn serialized_size_limit_cannot_poison_workspace() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "大页面");
    let blocks: Vec<_> = (0..100)
        .map(|i| json!({"id":format!("b{i}"),"type":"paragraph","text":""}))
        .collect();
    let mut p = json!({"title":"大小边界","blocks":blocks});
    let overhead = serde_json::to_vec(&p).unwrap().len();
    p["blocks"][0]["text"] = json!("x".repeat((16 << 20) - overhead - 500));
    assert!(serde_json::to_vec(&p).unwrap().len() < 16 << 20);
    assert!(serde_json::to_vec_pretty(&p).unwrap().len() > 16 << 20);
    assert!(s
        .dispatch("save", json!({"workspaceId":w,"page":p}))
        .is_err());
    s.dispatch("save", json!({"workspaceId":w,"page":{"title":"仍可写作"}}))
        .unwrap();
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn oversized_remote_page_is_rejected_before_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let remote = tempfile::tempdir().unwrap();
    git2::Repository::init_bare(remote.path()).unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = s
        .dispatch(
            "createWorkspace",
            json!({"name":"远程验证","remote":remote.path().to_str().unwrap()}),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let p = s
        .dispatch("save", json!({"workspaceId":w,"page":{"title":"可读页面"}}))
        .unwrap();
    s.dispatch("sync", json!({"workspaceId":w})).unwrap();
    let copy = tempfile::tempdir().unwrap();
    let repo = git2::build::RepoBuilder::new()
        .branch("main")
        .clone(remote.path().to_str().unwrap(), copy.path())
        .unwrap();
    let mut changed = p.clone();
    changed["blocks"] = json!([{"id":"b","type":"paragraph","text":"x".repeat(16<<20)}]);
    std::fs::write(
        copy.path()
            .join(format!("pages/{}.json", p["id"].as_str().unwrap())),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_all(["pages"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap();
    let sig = git2::Signature::now("测试", "local@test.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "远程超大文件", &tree, &[&parent])
        .unwrap();
    repo.find_remote("origin")
        .unwrap()
        .push(&["refs/heads/main:refs/heads/main"], None)
        .unwrap();
    assert!(s.dispatch("sync", json!({"workspaceId":w})).is_err());
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages[0]["title"], "可读页面");
}

#[test]
fn cyclic_backup_is_rejected_without_partial_pages() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "层级安全");
    let data = zip_files(&[
        ("manifest.json", "{\"format\":\"qingjian\",\"version\":1}"),
        (
            "pages/a.json",
            "{\"id\":\"a\",\"title\":\"甲\",\"parentId\":\"b\",\"blocks\":[]}",
        ),
        (
            "pages/b.json",
            "{\"id\":\"b\",\"title\":\"乙\",\"parentId\":\"a\",\"blocks\":[]}",
        ),
    ]);
    assert!(s.dispatch("import",json!({"workspaceId":w,"name":"cyclic.zip","data":base64::engine::general_purpose::STANDARD.encode(data)})).is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w})).unwrap(),
        json!([])
    );
}
#[test]
fn unnamed_mobile_image_attachment_keeps_image_extension() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "手机图片");
    let image = include_bytes!("../../../src-tauri/icons/32x32.png");
    let asset=s.dispatch("attach",json!({"workspaceId":w,"name":"123","data":base64::engine::general_purpose::STANDARD.encode(image)})).unwrap();
    assert!(asset["name"].as_str().unwrap().ends_with(".png"));
}

#[test]
fn html_import_preserves_text_without_active_scripts() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "网页迁移");
    let input="<html><head><style>.bad{}</style></head><body><h1>中文笔记</h1><p>正常<strong>正文</strong></p><script>alert('bad')</script><ul><li>事项</li></ul></body></html>";
    s.dispatch("import",json!({"workspaceId":w,"name":"笔记.html","data":base64::engine::general_purpose::STANDARD.encode(input)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages[0]["title"], "中文笔记");
    let blocks = pages[0]["blocks"].to_string();
    assert!(blocks.contains("正文"));
    assert!(blocks.contains("事项"));
    assert!(!blocks.contains("alert"));
    assert!(!blocks.contains(".bad"));
}

#[test]
fn zip_attachment_survives_backup_as_original_bytes() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let a = create(&mut s, "压缩附件源");
    let bytes = zip_files(&[("说明.txt", "压缩包内容")]);
    let asset=s.dispatch("attach",json!({"workspaceId":a,"name":"附件.zip","data":base64::engine::general_purpose::STANDARD.encode(&bytes)})).unwrap();
    s.dispatch("save",json!({"workspaceId":a,"page":{"title":"文件页面","blocks":[{"id":"b","type":"paragraph","text":format!("[压缩附件](asset:{})",asset["name"].as_str().unwrap())}]}})).unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":a})).unwrap();
    let b = create(&mut s, "压缩附件目标");
    s.dispatch(
        "import",
        json!({"workspaceId":b,"name":backup["name"],"data":backup["data"]}),
    )
    .unwrap();
    let restored = s
        .dispatch("asset", json!({"workspaceId":b,"name":asset["name"]}))
        .unwrap();
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(restored["data"].as_str().unwrap())
            .unwrap(),
        bytes
    );
}

#[test]
fn database_relations_follow_new_page_ids_after_backup_import() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "关联导入");
    let bytes=zip_files(&[("manifest.json","{\"format\":\"qingjian\",\"version\":1}"),("pages/tasks.json","{\"id\":\"tasks\",\"title\":\"任务\",\"blocks\":[],\"database\":{\"columns\":[{\"id\":\"name\",\"name\":\"任务\",\"type\":\"text\"}],\"rows\":[{\"id\":\"t1\",\"values\":{\"name\":\"设计\"}}]}}"),("pages/projects.json","{\"id\":\"projects\",\"title\":\"项目\",\"blocks\":[],\"database\":{\"columns\":[{\"id\":\"name\",\"name\":\"项目\",\"type\":\"text\"},{\"id\":\"tasks\",\"name\":\"任务\",\"type\":\"relation\",\"targetDatabaseId\":\"tasks\"}],\"rows\":[{\"id\":\"p1\",\"values\":{\"name\":\"轻笺\",\"tasks\":\"[\\\"t1\\\"]\"}}]}}")]);
    s.dispatch("import",json!({"workspaceId":w,"name":"backup.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    let tasks = ps.iter().find(|p| p["title"] == "任务").unwrap();
    let projects = ps.iter().find(|p| p["title"] == "项目").unwrap();
    assert_eq!(
        projects["database"]["columns"][1]["targetDatabaseId"],
        tasks["id"]
    );
    assert_eq!(
        projects["database"]["rows"][0]["values"]["tasks"],
        "[\"t1\"]"
    );
}
#[test]
fn markdown_simple_table_imports_as_editable_block() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "表格导入");
    let input = "# 表格\n\n| 名称 | 说明 |\n| --- | --- |\n| 轻笺 | 中文内容 |\n\n后续正文";
    s.dispatch("import",json!({"workspaceId":w,"name":"table.md","data":base64::engine::general_purpose::STANDARD.encode(input)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages[0]["blocks"][0]["type"], "table");
    assert!(pages[0]["blocks"][0]["text"]
        .as_str()
        .unwrap()
        .contains("轻笺"));
    assert_eq!(pages[0]["blocks"][1]["type"], "paragraph");
}

#[test]
fn malformed_advanced_property_configuration_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let w = create(&mut s, "属性配置");
    for column in [
        json!({"id":"x","name":"多选","type":"multiselect","options":{}}),
        json!({"id":"x","name":"公式","type":"formula","formula":1}),
        json!({"id":"x","name":"关联","type":"relation","targetDatabaseId":"../../path"}),
    ] {
        assert!(s.dispatch("save",json!({"workspaceId":w,"page":{"title":"配置","database":{"columns":[column],"rows":[]}}})).is_err());
    }
}

#[test]
fn notion_csv_properties_and_database_children_are_preserved() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "影视导入");
    let stem = "影视剧 1234567890abcdef1234567890abcdef";
    let data = zip_files(&[
        (&format!("{stem}_all.csv"), "电影名称,是否观看,观看时间,类型,地区,优先级,平台,备注\n牧神记,观看中,2025/10/15,动漫,中国🇨🇳,-1,bilibili,一段说明\n疾速追杀,未观看,,\"电影, 动漫\",美国🇺🇸,3,NAS,仍然是文字\n"),
        (&format!("{stem}/牧神记 abcdef1234567890abcdef1234567890.md"), "# 牧神记\n\n正文不能丢失"),
    ]);
    let report = s.dispatch("import", json!({"workspaceId":w,"name":"影视.zip","data":base64::engine::general_purpose::STANDARD.encode(data)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let db = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["database"].is_object())
        .unwrap();
    assert_eq!(db["title"], "影视剧");
    let cols = &db["database"]["columns"];
    assert_eq!(cols[1]["type"], "status");
    assert_eq!(cols[1]["options"], json!(["观看中", "未观看"]));
    assert_eq!(cols[1]["optionColors"]["观看中"], "blue");
    assert_eq!(cols[1]["optionColors"]["未观看"], "red");
    assert_eq!(cols[2]["type"], "date");
    assert_eq!(cols[3]["type"], "multiselect");
    assert_eq!(cols[4]["type"], "select");
    assert_eq!(cols[5]["type"], "number");
    assert_eq!(cols[6]["type"], "select");
    assert_eq!(cols[7]["type"], "text");
    assert_eq!(db["database"]["rows"][0]["values"]["col2"], "2025-10-15");
    assert_eq!(
        db["database"]["rows"][1]["values"]["col3"],
        "[\"电影\",\"动漫\"]"
    );
    assert_eq!(
        db["database"]["rows"][1]["sourceValues"]["col3"],
        "电影, 动漫"
    );
    let child = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "牧神记")
        .unwrap();
    assert_eq!(child["parentId"], db["id"]);
    assert_eq!(child["blocks"][0]["text"], "正文不能丢失");
    assert!(report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().unwrap().contains("推断")));
}

fn revisions(pages: &serde_json::Value) -> serde_json::Value {
    serde_json::Value::Object(
        pages
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p["id"].as_str().unwrap().to_owned(), p["revision"].clone()))
            .collect(),
    )
}
#[test]
fn bulk_trash_validates_all_targets_and_includes_children_once() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "批量回收");
    let parent = s
        .dispatch("save", json!({"workspaceId":w,"page":{"title":"父页面"}}))
        .unwrap();
    let child = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"子页面","parentId":parent["id"]}}),
        )
        .unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert!(s.dispatch("trashMany",json!({"workspaceId":w,"ids":[parent["id"],"missing"],"deleted":true,"revisions":revisions(&pages)})).is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w})).unwrap(),
        pages
    );
    let result=s.dispatch("trashMany",json!({"workspaceId":w,"ids":[parent["id"],child["id"]],"deleted":true,"revisions":revisions(&pages)})).unwrap();
    assert_eq!(result["count"], 2);
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert!(pages
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["deleted"] == true));
}
#[test]
fn permanent_trash_cleanup_preserves_shared_assets_and_requires_current_confirmation() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "空间清理");
    let shared=s.dispatch("attach",json!({"workspaceId":w,"name":"shared.png","data":base64::engine::general_purpose::STANDARD.encode(vec![1u8;4096])})).unwrap()["name"].as_str().unwrap().to_owned();
    let unique=s.dispatch("attach",json!({"workspaceId":w,"name":"unique.png","data":base64::engine::general_purpose::STANDARD.encode(vec![2u8;8192])})).unwrap()["name"].as_str().unwrap().to_owned();
    let active=s.dispatch("save",json!({"workspaceId":w,"page":{"title":"保留","blocks":[{"id":"b","type":"paragraph","text":format!("[附件](asset:{shared})")}]}})).unwrap();
    let trash=s.dispatch("save",json!({"workspaceId":w,"page":{"title":"删除","blocks":[{"id":"b","type":"paragraph","text":format!("[附件](asset:{shared}) [附件](asset:{unique})")}]}})).unwrap();
    s.dispatch(
        "trash",
        json!({"workspaceId":w,"id":trash["id"],"deleted":true}),
    )
    .unwrap();
    assert!(s
        .dispatch(
            "purgePreview",
            json!({"workspaceId":w,"ids":[active["id"]]})
        )
        .is_err());
    let plan = s
        .dispatch("purgePreview", json!({"workspaceId":w,"all":true}))
        .unwrap();
    assert_eq!(plan["assets"], json!([unique]));
    assert!(s
        .dispatch("purgeTrash", json!({"workspaceId":w,"preview":plan}))
        .is_err());
    let mut changed = active.clone();
    changed["title"] = json!("更新后的保留页");
    s.dispatch("save", json!({"workspaceId":w,"page":changed}))
        .unwrap();
    assert!(s
        .dispatch(
            "purgeTrash",
            json!({"workspaceId":w,"preview":plan,"confirmed":true})
        )
        .is_err());
    let plan = s
        .dispatch("purgePreview", json!({"workspaceId":w,"all":true}))
        .unwrap();
    let before = s.dispatch("stats", json!({"workspaceId":w})).unwrap()["total"]
        .as_u64()
        .unwrap();
    let result = s
        .dispatch(
            "purgeTrash",
            json!({"workspaceId":w,"preview":plan,"confirmed":true}),
        )
        .unwrap();
    assert_eq!(result["pages"], 1);
    assert_eq!(result["assets"], 1);
    assert!(result["bytes"].as_u64().unwrap() > 8192);
    let after = s.dispatch("stats", json!({"workspaceId":w})).unwrap()["total"]
        .as_u64()
        .unwrap();
    assert_eq!(before - after, result["bytes"].as_u64().unwrap());
    assert!(s
        .dispatch("asset", json!({"workspaceId":w,"name":shared}))
        .is_ok());
    assert!(s
        .dispatch("asset", json!({"workspaceId":w,"name":unique}))
        .is_err());
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn history_restore_recovers_an_attachment_removed_by_trash_cleanup() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "历史附件");
    let name=s.dispatch("attach",json!({"workspaceId":w,"name":"old.png","data":base64::engine::general_purpose::STANDARD.encode(b"old-image")})).unwrap()["name"].as_str().unwrap().to_owned();
    let mut page=s.dispatch("save",json!({"workspaceId":w,"page":{"title":"保留历史页","blocks":[{"id":"b","type":"paragraph","text":format!("[附件](asset:{name})")}]}})).unwrap();
    let commit = s.dispatch("commit", json!({"workspaceId":w})).unwrap()["commit"].clone();
    page["blocks"] = json!([]);
    let page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    let removed=s.dispatch("save",json!({"workspaceId":w,"page":{"title":"回收页","blocks":[{"id":"b","type":"paragraph","text":format!("[附件](asset:{name})")}]}})).unwrap();
    s.dispatch(
        "trash",
        json!({"workspaceId":w,"id":removed["id"],"deleted":true}),
    )
    .unwrap();
    let plan = s
        .dispatch("purgePreview", json!({"workspaceId":w,"all":true}))
        .unwrap();
    s.dispatch(
        "purgeTrash",
        json!({"workspaceId":w,"preview":plan,"confirmed":true}),
    )
    .unwrap();
    assert!(s
        .dispatch("asset", json!({"workspaceId":w,"name":name}))
        .is_err());
    s.dispatch(
        "restore",
        json!({"workspaceId":w,"id":page["id"],"commit":commit}),
    )
    .unwrap();
    assert!(s
        .dispatch("asset", json!({"workspaceId":w,"name":name}))
        .is_ok());
}
#[test]
fn bulk_trash_does_not_rewrite_children_already_in_trash() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "保留已回收页");
    let parent = s
        .dispatch("save", json!({"workspaceId":w,"page":{"title":"父"}}))
        .unwrap();
    let child = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"子","parentId":parent["id"]}}),
        )
        .unwrap();
    s.dispatch(
        "trash",
        json!({"workspaceId":w,"id":child["id"],"deleted":true}),
    )
    .unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let before = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == child["id"])
        .unwrap()
        .clone();
    let result=s.dispatch("trashMany",json!({"workspaceId":w,"ids":[parent["id"]],"revisions":revisions(&pages),"deleted":true})).unwrap();
    assert_eq!(result["count"], 1);
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w}))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == child["id"])
            .unwrap(),
        &before
    );
}

#[test]
fn notion_export_plain_folder_and_full_csv_form_one_database_parent() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "真实结构样本");
    let bytes=zip_files(&[
 ("私人与共享/影视剧 1234567890abcdef1234567890abcdef.csv","名称,是否观看,类型\n同名,观看中,动漫\n同名,未观看,电视剧\n"),
 ("私人与共享/影视剧 1234567890abcdef1234567890abcdef_all.csv","名称,描述,是否观看,类型\n同名,文字二,未观看,电视剧\n同名,文字一,观看中,动漫\n"),
 ("私人与共享/影视剧/同名 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.md","# 同名\n\n是否观看: 观看中\n类型: 动漫\n\n正文一"),
 ("私人与共享/影视剧/同名 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.md","# 同名\n\n是否观看: 未观看\n类型: 电视剧\n\n正文二"),
 ("入口.md","# 入口\n\n[影视](私人与共享/影视剧%201234567890abcdef1234567890abcdef.csv)\n[全部](私人与共享/影视剧%201234567890abcdef1234567890abcdef_all.csv)")]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"导出.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    assert_eq!(report["pages"], 4);
    let parents: Vec<_> = ps.iter().filter(|p| p["database"].is_object()).collect();
    assert_eq!(parents.len(), 1);
    let parent = parents[0];
    assert_eq!(parent["title"], "影视剧");
    assert_eq!(
        parent["database"]["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["名称", "是否观看", "类型", "描述"]
    );
    let children: Vec<_> = ps
        .iter()
        .filter(|p| p["parentId"] == parent["id"])
        .collect();
    assert_eq!(children.len(), 2);
    assert!(children.iter().all(|p| p["title"] == "同名"));
    assert_ne!(children[0]["id"], children[1]["id"]);
    for row in parent["database"]["rows"].as_array().unwrap() {
        let child = children.iter().find(|p| p["id"] == row["pageId"]).unwrap();
        assert!(child["blocks"]
            .to_string()
            .contains(row["sourceValues"]["col2"].as_str().unwrap()));
    }
    let entry = ps.iter().find(|p| p["title"] == "入口").unwrap();
    assert_eq!(
        entry["blocks"]
            .to_string()
            .matches(&format!("page:{}", parent["id"].as_str().unwrap()))
            .count(),
        2
    );
}
#[test]
fn notion_parent_alias_ambiguity_is_reported_without_guessing() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "歧义目录");
    let data = zip_files(&[
        ("目录/页面 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.md", "# 甲"),
        ("目录/页面 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.md", "# 乙"),
        ("目录/页面/子.md", "# 子"),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"目录.zip","data":base64::engine::general_purpose::STANDARD.encode(data)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let child = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "子")
        .unwrap();
    assert!(child["parentId"].as_str().unwrap_or("").is_empty());
    assert!(report["warnings"].to_string().contains("多个"));
}
#[test]
#[ignore = "真实用户导出文件仅在本机显式验证，不提交样本"]
fn actual_notion_export_has_one_root_and_sixty_three_children() {
    use base64::Engine;
    let path = std::env::var("QINGJIAN_TEST_NOTION_ZIP").expect("请设置真实导出文件路径");
    let bytes = std::fs::read(path).unwrap();
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "真实导出隔离验证");
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"影视导出.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    assert_eq!(report["pages"], 64);
    let roots: Vec<_> = ps
        .iter()
        .filter(|p| p["parentId"].as_str().unwrap_or("").is_empty())
        .collect();
    assert_eq!(roots.len(), 1);
    let parent = roots[0];
    assert_eq!(parent["title"], "影视剧");
    assert_eq!(parent["database"]["rows"].as_array().unwrap().len(), 63);
    assert_eq!(
        ps.iter().filter(|p| p["parentId"] == parent["id"]).count(),
        63
    );
    for r in parent["database"]["rows"].as_array().unwrap() {
        assert!(ps
            .iter()
            .any(|p| p["id"] == r["pageId"] && p["parentId"] == parent["id"]));
    }
    assert_eq!(ps.iter().filter(|p| p["title"] == "少年歌行").count(), 2);
}
#[test]
fn backup_remaps_database_row_page_links() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "关联页面备份");
    let parent=s.dispatch("save",json!({"workspaceId":w,"page":{"title":"父数据库","database":{"columns":[{"id":"name","name":"名称","type":"text"}],"rows":[]}}})).unwrap();
    let child = s
        .dispatch(
            "save",
            json!({"workspaceId":w,"page":{"title":"子页面","parentId":parent["id"]}}),
        )
        .unwrap();
    let mut parent = parent;
    parent["database"]["rows"] =
        json!([{"id":"r","values":{"name":"子页面"},"pageId":child["id"]}]);
    s.dispatch("save", json!({"workspaceId":w,"page":parent}))
        .unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let other = create(&mut s, "恢复");
    s.dispatch(
        "import",
        json!({"workspaceId":other,"name":"备份.zip","data":backup["data"]}),
    )
    .unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":other})).unwrap();
    let ps = pages.as_array().unwrap();
    let parent = ps.iter().find(|p| p["database"].is_object()).unwrap();
    let child = ps.iter().find(|p| p["title"] == "子页面").unwrap();
    assert_eq!(parent["database"]["rows"][0]["pageId"], child["id"]);
    assert_eq!(child["parentId"], parent["id"]);
}
#[test]
fn different_csv_variants_are_not_merged_and_exact_parent_wins() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "非同源表格");
    let data = zip_files(&[
        (
            "库 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.csv",
            "名称,状态\n甲,进行中",
        ),
        (
            "库 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa_all.csv",
            "名称,状态\n甲,已完成",
        ),
        ("库/子.md", "# 子"),
        ("库.md", "# 确切父页面"),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"表格.zip","data":base64::engine::general_purpose::STANDARD.encode(data)})).unwrap();
    assert_eq!(report["pages"], 4);
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    let exact = ps.iter().find(|p| p["title"] == "确切父页面").unwrap();
    let child = ps.iter().find(|p| p["title"] == "子").unwrap();
    assert_eq!(child["parentId"], exact["id"]);
}

#[test]
fn notion_html_database_and_csv_restore_types_colors_children_and_body() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "网页结构");
    let parent = r#"<html><head><style>不要作为正文</style><meta name="data-notion-page-icon" content="🗓️"></head><body><article id="aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa" class="page"><header><h1 class="page-title">影视剧</h1><p class="page-description">观影记录</p></header><div class="page-body"><p>父页面介绍</p><table class="collection-content"><thead><tr><th><img src="font_gray.svg">名称</th><th><img src="burst_gray.svg">是否观看</th><th><img src="list_gray.svg">类型</th><th><img src="calendar_gray.svg">观看时间</th></tr></thead><tbody><tr id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb"><td class="cell-title"><a href="影视剧/同名%20bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html" data-notion-page-id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb">同名</a></td><td><span class="status-value select-value-color-blue">观看中</span></td><td><span class="selected-value select-value-color-green">动漫</span></td><td><time datetime="2025-10-15">2025/10/15</time></td></tr><tr id="cccccccc-cccc-cccc-cccc-cccccccccccc"><td class="cell-title"><a href="影视剧/同名%20cccccccccccccccccccccccccccccccc.html">同名</a></td><td><span class="status-value select-value-color-red">未观看</span></td><td><span class="selected-value select-value-color-pink">电影</span><span class="selected-value select-value-color-green">动漫</span></td><td></td></tr></tbody></table></div></article><script>恶意脚本</script></body></html>"#;
    let a = r#"<article id="bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb" class="page"><header><h1 class="page-title">同名</h1><table class="properties"><tr class="property-row property-row-number"><th>评分</th><td>8</td></tr><tr class="property-row property-row-multi_select"><th>类型</th><td><span class="selected-value select-value-color-green">动漫</span></td></tr></table></header><div class="page-body"><p>正文一 <strong>重要内容</strong></p><a href="../影视剧%20aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.html">返回父级</a><script>恶意脚本</script></div></article>"#;
    let b = r#"<article id="cccccccc-cccc-cccc-cccc-cccccccccccc" class="page"><header><h1 class="page-title">同名</h1><div class="properties"></div></header><div class="page-body"><p>正文二</p></div></article>"#;
    let bytes=zip_files(&[("影视剧 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.html",parent),("影视剧 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.csv","名称,评分,是否观看,类型,观看时间\n同名,8,观看中,动漫,2025/10/15\n同名,6,未观看,\"电影, 动漫\",\n"),("影视剧/同名 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html",a),("影视剧/同名 cccccccccccccccccccccccccccccccc.html",b)]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"网页.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    assert_eq!(report["pages"], 3);
    let roots: Vec<_> = ps
        .iter()
        .filter(|p| p["parentId"].as_str().unwrap_or("").is_empty())
        .collect();
    assert_eq!(roots.len(), 1);
    let root = roots[0];
    assert_eq!(root["icon"], "🗓️");
    let cols = root["database"]["columns"].as_array().unwrap();
    let status = cols.iter().find(|c| c["name"] == "是否观看").unwrap();
    assert_eq!(status["type"], "status");
    assert_eq!(status["optionColors"]["观看中"], "blue");
    assert_eq!(status["optionColors"]["未观看"], "red");
    let types = cols.iter().find(|c| c["name"] == "类型").unwrap();
    assert_eq!(types["type"], "multiselect");
    assert_eq!(types["optionColors"]["电影"], "pink");
    assert_eq!(
        cols.iter().find(|c| c["name"] == "评分").unwrap()["type"],
        "number"
    );
    assert!(root["blocks"].to_string().contains("父页面介绍"));
    assert!(!root["blocks"].to_string().contains("font_gray"));
    assert!(!root["blocks"].to_string().contains("恶意脚本"));
    assert_eq!(root["database"]["rows"].as_array().unwrap().len(), 2);
    for row in root["database"]["rows"].as_array().unwrap() {
        let child = ps.iter().find(|p| p["id"] == row["pageId"]).unwrap();
        assert_eq!(child["parentId"], root["id"]);
        assert!(child["blocks"].to_string().contains("正文"));
        assert!(!child["blocks"].to_string().contains("恶意脚本"));
    }
    let child = ps
        .iter()
        .find(|p| p["blocks"].to_string().contains("正文一"))
        .unwrap();
    assert!(child["blocks"]
        .to_string()
        .contains(&format!("page:{}", root["id"].as_str().unwrap())));
}
#[test]
#[ignore = "真实网页导出仅在本机显式验证，不提交原文件"]
fn actual_html_notion_export_has_one_database_and_original_colors() {
    use base64::Engine;
    let bytes =
        std::fs::read(std::env::var("QINGJIAN_TEST_HTML_ZIP").expect("请设置原始网页压缩包路径"))
            .unwrap();
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "网页真实验证");
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"网页.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    assert_eq!(report["pages"], 64);
    let roots: Vec<_> = ps
        .iter()
        .filter(|p| p["parentId"].as_str().unwrap_or("").is_empty())
        .collect();
    assert_eq!(roots.len(), 1);
    let root = roots[0];
    assert_eq!(root["title"], "影视剧");
    assert_eq!(root["icon"], "🗓️");
    let cols = root["database"]["columns"].as_array().unwrap();
    assert_eq!(cols.len(), 11);
    assert_eq!(
        cols.iter().find(|c| c["name"] == "类型").unwrap()["type"],
        "multiselect"
    );
    let status = cols.iter().find(|c| c["name"] == "是否观看").unwrap();
    assert_eq!(status["optionColors"]["已看完"], "green");
    assert_eq!(
        cols.iter().find(|c| c["name"] == "平台").unwrap()["optionColors"]["NAS"],
        "brown"
    );
    let rows = root["database"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 63);
    for row in rows {
        assert!(ps
            .iter()
            .any(|p| p["id"] == row["pageId"] && p["parentId"] == root["id"]));
    }
    assert_eq!(ps.iter().filter(|p| p["title"] == "少年歌行").count(), 2);
    assert!(!pages.to_string().contains("font_gray.svg"));
}
#[test]
fn html_conflicting_child_property_does_not_overwrite_csv_value() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "属性冲突");
    let root = r#"<article class="page" id="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"><h1 class="page-title">父</h1><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th></tr></thead><tbody><tr id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><td><a href="父/子 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html">子</a></td></tr></tbody></table></div></article>"#;
    let child = r#"<article class="page" id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><h1 class="page-title">子</h1><table class="properties"><tr class="property-row property-row-multi_select"><th>分类</th><td><span class="selected-value select-value-color-pink">不同分类</span></td></tr></table><div class="page-body"><p>正文</p></div></article>"#;
    let bytes = zip_files(&[
        ("父 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.html", root),
        (
            "父 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.csv",
            "名称,分类\n子,原始分类",
        ),
        ("父/子 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html", child),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"冲突.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let parent = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["database"].is_object())
        .unwrap();
    assert_eq!(
        parent["database"]["rows"][0]["values"]["col1"],
        "[\"原始分类\"]"
    );
    assert!(report["warnings"].to_string().contains("冲突"));
    assert!(pages.to_string().contains("不同分类"));
}
#[test]
fn html_mismatched_record_identity_is_not_relinked_by_title() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "链接标识冲突");
    let root = r#"<article class="page" id="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"><h1 class="page-title">父</h1><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th></tr></thead><tbody><tr id="cccccccccccccccccccccccccccccccc"><td><a href="父/子 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html">子</a></td></tr></tbody></table></div></article>"#;
    let child = r#"<article class="page" id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><h1 class="page-title">子</h1><div class="page-body"><p>正文</p></div></article>"#;
    let bytes = zip_files(&[
        ("父 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.html", root),
        ("父/子 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html", child),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"标识.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let root = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["database"].is_object())
        .unwrap();
    assert!(root["database"]["rows"][0]["pageId"].is_null());
    assert!(report["warnings"].to_string().contains("引用无法唯一"));
}
#[test]
fn html_source_identity_conflict_prevents_csv_merge() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "父标识冲突");
    let html = r#"<article class="page" id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><h1 class="page-title">父</h1><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th></tr></thead><tbody><tr><td>记录</td></tr></tbody></table></div></article>"#;
    let bytes = zip_files(&[
        ("父 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.html", html),
        (
            "父 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.csv",
            "名称,备注\n记录,完整字段",
        ),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"源.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    assert_eq!(report["pages"], 2);
    assert!(report["warnings"].to_string().contains("标识"));
}

#[test]
fn html_database_and_properties_preserve_attachments_and_custom_links() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "网页附件");
    let html = r#"<article class="page"><h1 class="page-title">资料</h1><table class="properties"><tr class="property-row-text"><th>参考</th><td><a href="附件/说明.txt">原始说明</a></td></tr></table><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th><th>附件</th><th>图片</th><th>链接</th></tr></thead><tbody><tr><td>甲</td><td><a href="附件/文档.pdf">查看文档</a></td><td><img src="附件/海报.png" alt="海报"></td><td><a href="https://example.com/detail">详情</a></td></tr></tbody></table></div></article>"#;
    let bytes = zip_files(&[
        ("资料.html", html),
        ("资料.csv", "名称,附件,图片,链接\n甲,查看文档,,详情"),
        ("附件/文档.pdf", "原始文档"),
        ("附件/海报.png", "原始图片"),
        ("附件/说明.txt", "原始说明"),
    ]);
    s.dispatch("import",json!({"workspaceId":w,"name":"附件.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1);
    let body = pages[0]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["text"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("[详情](https://example.com/detail)"));
    assert!(body.contains("查看文档"));
    let references = regex::Regex::new(r"asset:([A-Za-z0-9_.-]+)").unwrap();
    let mut content = vec![];
    for item in references.captures_iter(&body) {
        let asset = s
            .dispatch("asset", json!({"workspaceId":w,"name":&item[1]}))
            .unwrap();
        content.push(
            String::from_utf8(
                base64::engine::general_purpose::STANDARD
                    .decode(asset["data"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap(),
        );
    }
    content.sort();
    assert_eq!(content, vec!["原始图片", "原始文档", "原始说明"]);
}

#[test]
fn html_large_numbers_match_csv_without_precision_loss() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "精确标识");
    let html = r#"<article class="page"><h1 class="page-title">记录</h1><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th><th><img src="hashtag_gray.svg">编号</th></tr></thead><tbody><tr id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><td>同名</td><td>9007199254740992</td></tr><tr id="cccccccccccccccccccccccccccccccc"><td>同名</td><td>9007199254740993</td></tr></tbody></table></div></article>"#;
    let bytes = zip_files(&[
        ("记录.html", html),
        (
            "记录.csv",
            "名称,编号,备注\n同名,9007199254740993,第二条\n同名,9007199254740992,第一条",
        ),
    ]);
    s.dispatch("import",json!({"workspaceId":w,"name":"数字.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages.as_array().unwrap().len(), 1);
    let rows = pages[0]["database"]["rows"].as_array().unwrap();
    assert_eq!(rows[0]["sourceId"], "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    assert_eq!(rows[0]["values"]["col2"], "第一条");
    assert_eq!(rows[1]["sourceId"], "cccccccccccccccccccccccccccccccc");
    assert_eq!(rows[1]["values"]["col2"], "第二条");
}

#[test]
fn html_identical_visible_records_do_not_guess_hidden_csv_fields() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "同名歧义");
    let html = r#"<article class="page"><h1 class="page-title">记录</h1><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th></tr></thead><tbody><tr id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><td>同名</td></tr><tr id="cccccccccccccccccccccccccccccccc"><td>同名</td></tr></tbody></table></div></article>"#;
    let bytes = zip_files(&[
        ("记录.html", html),
        ("记录.csv", "名称,备注\n同名,第二条\n同名,第一条"),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"歧义.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    assert_eq!(report["pages"], 2);
    assert!(report["warnings"].to_string().contains("分别保留"));
}

#[test]
fn database_metadata_and_monotonic_sequence_survive_layout_and_tampering() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "记录元数据");
    let mut page = s.dispatch("save", json!({"workspaceId":w,"page":{"title":"数据库","database":{"columns":[{"id":"name","name":"名称","type":"text"}],"rows":[{"id":"first","values":{"name":"原值"}},{"id":"second","values":{"name":"另一个"}}]}}})).unwrap();
    assert_eq!(page["createdBy"], "本机用户");
    assert_eq!(page["database"]["rows"][0]["sequence"], 1);
    assert_eq!(page["database"]["rows"][1]["sequence"], 2);
    assert_eq!(page["database"]["nextSequence"], 3);
    let metadata = page["database"]["rows"][0].clone();
    page["database"]["columns"][0]["name"] = json!("新名称");
    page["database"]["rows"][0]["createdBy"] = json!("伪造作者");
    page["database"]["rows"][0]["sequence"] = json!(90);
    page["database"]["nextSequence"] = json!(1);
    page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    assert_eq!(
        page["database"]["rows"][0]["createdBy"],
        metadata["createdBy"]
    );
    assert_eq!(page["database"]["rows"][0]["sequence"], 1);
    assert_eq!(
        page["database"]["rows"][0]["updatedAt"],
        metadata["updatedAt"]
    );
    page["database"]["rows"].as_array_mut().unwrap().remove(1);
    page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    page["database"]["rows"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"third","values":{"name":"新增"}}));
    page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    assert_eq!(page["database"]["rows"][1]["sequence"], 3);
    assert_eq!(page["database"]["nextSequence"], 4);
    page["database"]["rows"][0]["values"]["name"] = json!("修改值");
    page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    assert_ne!(
        page["database"]["rows"][0]["updatedAt"],
        metadata["updatedAt"]
    );
    assert_eq!(page["database"]["rows"][0]["updatedBy"], "本机用户");
}

#[test]
fn typed_property_validation_prevents_broken_media_location_and_buttons() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "属性校验");
    for (kind, value) in [
        ("person", "[1]"),
        ("files", r#"[{"name":"图","asset":"asset:../../escape"}]"#),
        ("location", r#"{"address":"北京","latitude":100}"#),
    ] {
        let result = s.dispatch("save", json!({"workspaceId":w,"page":{"database":{"columns":[{"id":"name","name":"名称","type":kind}],"rows":[{"id":"row","values":{"name":value}}]}}}));
        assert!(result.is_err(), "{kind}");
    }
    let result = s.dispatch("save", json!({"workspaceId":w,"page":{"database":{"columns":[{"id":"name","name":"名称","type":"text"},{"id":"button","name":"操作","type":"button","buttonAction":{"kind":"script","propertyId":"name","value":"bad"}}],"rows":[]}}}));
    assert!(result.is_err());
    let valid = s.dispatch("save", json!({"workspaceId":w,"page":{"database":{"columns":[{"id":"title","name":"名称","type":"text"},{"id":"name","name":"人员","type":"person"},{"id":"files","name":"媒体","type":"files"},{"id":"location","name":"位置","type":"location"},{"id":"button","name":"操作","type":"button","buttonAction":{"kind":"set","propertyId":"name","value":"[]"},"buttonLabel":"设为空"},{"id":"uid","name":"编号","type":"uniqueId","uniqueIdPrefix":"记录"}],"rows":[{"id":"row","values":{"name":"[\"张三\"]","files":"[{\"name\":\"图\",\"asset\":\"asset:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.png\"}]","location":"{\"address\":\"北京\",\"latitude\":39.9,\"longitude\":116.4}"}}]}}}));
    assert!(valid.is_ok(), "{valid:?}");
}

#[test]
fn notion_html_explicit_new_property_types_and_original_authors_are_preserved() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "新类型导入");
    let data = zip_files(&[
        (
            "库 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.html",
            r#"<article class="page" id="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"><h1 class="page-title">库</h1><div class="page-body"><table class="collection-content"><thead><tr><th>名称</th><th>人员</th><th>媒体</th><th>创建时间</th><th>创建者</th><th>邮箱</th><th>电话</th></tr></thead><tbody><tr id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><td><a href="库/子 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html">子</a></td><td>甲</td><td>海报</td><td>2024-01-02</td><td>原作者</td><td>a@example.com</td><td>123</td></tr></tbody></table></div></article>"#,
        ),
        (
            "库/子 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.html",
            r#"<article class="page" id="bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"><h1 class="page-title">子</h1><table class="properties"><tr class="property-row-person"><th>人员</th><td><span class="user">甲</span></td></tr><tr class="property-row-files"><th>媒体</th><td><a href="海报.png">海报</a></td></tr><tr class="property-row-created_time"><th>创建时间</th><td><time datetime="2024-01-02">2024-01-02</time></td></tr><tr class="property-row-created_by"><th>创建者</th><td>原作者</td></tr><tr class="property-row-email"><th>邮箱</th><td>a@example.com</td></tr><tr class="property-row-phone_number"><th>电话</th><td>123</td></tr></table><div class="page-body"></div></article>"#,
        ),
        ("库/海报.png", "image"),
    ]);
    s.dispatch("import", json!({"workspaceId":w,"name":"html.zip","data":base64::engine::general_purpose::STANDARD.encode(data)})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let parent = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "库")
        .unwrap();
    let cols = parent["database"]["columns"].as_array().unwrap();
    assert_eq!(cols[1]["type"], "person");
    assert_eq!(cols[2]["type"], "files");
    assert_eq!(cols[3]["type"], "createdTime");
    assert_eq!(cols[4]["type"], "createdBy");
    assert_eq!(cols[5]["type"], "email");
    assert_eq!(cols[6]["type"], "phone");
    let row = &parent["database"]["rows"][0];
    assert_eq!(row["createdAt"], "2024-01-02");
    assert_eq!(row["createdBy"], "原作者");
    assert!(row["values"]["col2"].as_str().unwrap().contains("asset:"));
    let child = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "子")
        .unwrap();
    assert_eq!(child["createdBy"], "原作者");
}

#[test]
fn legacy_imported_metadata_is_initialized_without_layout_time_changes() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "旧数据升级");
    let backup = zip_files(&[
        ("manifest.json", r#"{"format":"qingjian","version":1}"#),
        (
            "pages/old.json",
            r#"{"id":"old","title":"旧数据","blocks":[],"createdAt":"2020-01-01","updatedAt":"2020-02-01","createdBy":"父页作者","database":{"columns":[{"id":"name","name":"名称","type":"text"},{"id":"creator","name":"创建者","type":"createdBy"}],"rows":[{"id":"row","values":{"name":"原值","creator":"记录作者"}}]}}"#,
        ),
    ]);
    s.dispatch("import", json!({"workspaceId":w,"name":"backup.zip","data":base64::engine::general_purpose::STANDARD.encode(backup)})).unwrap();
    let mut page = s.dispatch("pages", json!({"workspaceId":w})).unwrap()[0].clone();
    assert_eq!(page["createdAt"], "2020-01-01");
    assert_eq!(page["database"]["rows"][0]["createdBy"], "记录作者");
    assert_eq!(page["database"]["rows"][0]["updatedBy"], "");
    let before = page["database"]["rows"][0].clone();
    page["database"]["columns"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"empty","name":"新属性","type":"text"}));
    page["database"]["rows"][0]["values"]["empty"] = json!("");
    page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    assert_eq!(
        page["database"]["rows"][0]["updatedAt"],
        before["updatedAt"]
    );
    assert_eq!(page["database"]["rows"][0]["updatedBy"], "");
}

#[test]
fn local_new_rows_do_not_claim_imported_system_field_authors() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "本机创建");
    let p = s.dispatch("save", json!({"workspaceId":w,"page":{"createdBy":"伪造页面作者","database":{"columns":[{"id":"author","name":"创建者","type":"createdBy"}],"rows":[{"id":"row","values":{"author":"伪造记录作者"}}]}}})).unwrap();
    assert_eq!(p["createdBy"], "本机用户");
    assert_eq!(p["database"]["rows"][0]["createdBy"], "本机用户");
}

#[test]
fn new_properties_backup_keeps_media_bytes_metadata_and_unused_sequence() {
    use base64::Engine;
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::new(dir.path()).unwrap();
    let a = create(&mut s, "属性备份源");
    let asset = s.dispatch("attach", json!({"workspaceId":a,"name":"海报.png","data":base64::engine::general_purpose::STANDARD.encode("media bytes")})).unwrap();
    let file_value = serde_json::to_string(
        &json!([{"name":"海报","asset":format!("asset:{}",asset["name"].as_str().unwrap())}]),
    )
    .unwrap();
    let mut page = s.dispatch("save", json!({"workspaceId":a,"page":{"title":"完整属性","database":{"columns":[{"id":"files","name":"媒体","type":"files"},{"id":"name","name":"人员","type":"person"}],"rows":[{"id":"kept","values":{"files":file_value,"name":"[\"甲\",\"乙\"]"}},{"id":"removed","values":{}}]}}})).unwrap();
    page["database"]["rows"].as_array_mut().unwrap().remove(1);
    page = s
        .dispatch("save", json!({"workspaceId":a,"page":page}))
        .unwrap();
    let backup = s.dispatch("export", json!({"workspaceId":a})).unwrap();
    let b = create(&mut s, "属性备份目标");
    s.dispatch(
        "import",
        json!({"workspaceId":b,"name":backup["name"],"data":backup["data"]}),
    )
    .unwrap();
    let mut restored = s.dispatch("pages", json!({"workspaceId":b})).unwrap()[0].clone();
    assert_eq!(restored["database"]["rows"][0], page["database"]["rows"][0]);
    assert_eq!(restored["createdAt"], page["createdAt"]);
    assert_eq!(restored["database"]["nextSequence"], 3);
    let result = s
        .dispatch("asset", json!({"workspaceId":b,"name":asset["name"]}))
        .unwrap();
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(result["data"].as_str().unwrap())
            .unwrap(),
        b"media bytes"
    );
    restored["database"]["rows"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"new","values":{}}));
    restored = s
        .dispatch("save", json!({"workspaceId":b,"page":restored}))
        .unwrap();
    assert_eq!(restored["database"]["rows"][1]["sequence"], 3);
}

#[test]
fn database_row_body_updates_edit_metadata_but_column_layout_does_not() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "记录正文元数据");
    let mut page = s.dispatch("save", json!({"workspaceId":w,"page":{"database":{"columns":[{"id":"name","name":"名称","type":"text"}],"rows":[{"id":"row","values":{"name":"记录"},"body":"原正文"}]}}})).unwrap();
    let created = page["database"]["rows"][0].clone();
    page["database"]["rows"][0]["body"] = json!("修改正文");
    page = s
        .dispatch("save", json!({"workspaceId":w,"page":page}))
        .unwrap();
    assert_ne!(
        page["database"]["rows"][0]["updatedAt"],
        created["updatedAt"]
    );
    assert_eq!(
        page["database"]["rows"][0]["createdAt"],
        created["createdAt"]
    );
    assert_eq!(page["database"]["rows"][0]["updatedBy"], "本机用户");
}

#[test]
fn property_configuration_rejects_partial_coordinates() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "完整属性校验");
    for value in [
        r#"{"address":"北京","latitude":39.9}"#,
        r#"{"address":"北京","longitude":116.4}"#,
    ] {
        assert!(s.dispatch("save", json!({"workspaceId":w,"page":{"database":{"columns":[{"id":"name","name":"名称","type":"text"},{"id":"loc","name":"地点","type":"location"}],"rows":[{"id":"row","values":{"loc":value}}]}}})).is_err());
    }
}

#[test]
fn property_configuration_rejects_unsupported_button_targets() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "按钮目标校验");
    for (kind, target) in [
        ("files", "target"),
        ("location", "target"),
        ("unknown", "target"),
        ("text", "name"),
        ("button", "button"),
    ] {
        let result=s.dispatch("save", json!({"workspaceId":w,"page":{"database":{"columns":[{"id":"name","name":"名称","type":"text"},{"id":"target","name":"目标","type":kind},{"id":"button","name":"按钮","type":"button","buttonAction":{"kind":"set","propertyId":target,"value":""}}],"rows":[]}}}));
        assert!(result.is_err(), "{kind} {target}");
    }
}

#[test]
fn backup_remaps_file_properties_with_legacy_asset_names() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "附件字段重映射");
    let backup = zip_files(&[
        ("manifest.json", r#"{"format":"qingjian","version":1}"#),
        (
            "pages/parent.json",
            r#"{"id":"parent","title":"媒体","blocks":[],"database":{"columns":[{"id":"name","name":"名称","type":"text"},{"id":"files","name":"媒体","type":"files"}],"rows":[{"id":"row","values":{"name":"原图","files":"[{\"name\":\"原图\",\"asset\":\"asset:photo.png\",\"caption\":\"保留说明\"}]"}}]},"importProperties":[{"name":"媒体","type":"files","value":"[{\"name\":\"原图\",\"asset\":\"asset:photo.png\"}]"}]}"#,
        ),
        ("assets/photo.png", "original media bytes"),
    ]);
    s.dispatch("import", json!({"workspaceId":w,"name":"backup.zip","data":base64::engine::general_purpose::STANDARD.encode(backup)})).unwrap();
    let page = s.dispatch("pages", json!({"workspaceId":w})).unwrap()[0].clone();
    let files: serde_json::Value = serde_json::from_str(
        page["database"]["rows"][0]["values"]["files"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let prop_files: serde_json::Value =
        serde_json::from_str(page["importProperties"][0]["value"].as_str().unwrap()).unwrap();
    let reference = files[0]["asset"].as_str().unwrap();
    assert_ne!(reference, "asset:photo.png");
    let name = reference.strip_prefix("asset:").unwrap();
    assert!(name.ends_with(".png"));
    assert_eq!(files[0]["caption"], "保留说明");
    assert_eq!(prop_files[0]["asset"], files[0]["asset"]);
    let asset = s
        .dispatch("asset", json!({"workspaceId":w,"name":name}))
        .unwrap();
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(asset["data"].as_str().unwrap())
            .unwrap(),
        b"original media bytes"
    );
}

#[test]
fn notion_page_links_restore_icons_colors_and_backup_assets() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "页面条目");
    let parent = r#"<article class="page"><header><div class="page-header-icon"><img class="icon" src="icons/main.png"></div><h1 class="page-title">后端开发</h1></header><div class="page-body"><figure class="link-to-page block-color-purple_background"><a href="后端开发/数据库.html"><img class="icon" src="icons/db.png">数据库</a></figure><figure class="link-to-page block-color-green"><a href="后端开发/缓存.html"><span class="icon">🟢</span>缓存</a></figure></div></article>"#;
    let child = r#"<article class="page"><header><div class="page-header-icon"><span class="icon">🗃️</span></div><h1 class="page-title">数据库</h1></header><div class="page-body"><p>正文</p></div></article>"#;
    let bytes = zip_files(&[
        ("后端开发.html", parent),
        ("后端开发/数据库.html", child),
        ("后端开发/缓存.html", &child.replace("数据库", "缓存")),
        ("icons/main.png", "主图标"),
        ("icons/db.png", "子图标"),
    ]);
    let report=s.dispatch("import",json!({"workspaceId":w,"name":"条目.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)})).unwrap();
    assert_eq!(report["pages"], 3);
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let ps = pages.as_array().unwrap();
    let root = ps.iter().find(|p| p["title"] == "后端开发").unwrap();
    let child = ps.iter().find(|p| p["title"] == "数据库").unwrap();
    assert!(root["icon"].as_str().unwrap().starts_with("asset:"));
    assert_eq!(child["icon"], "🗃️");
    let blocks = root["blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0]["backgroundColor"], "purple");
    assert!(blocks[0]["linkIcon"]
        .as_str()
        .unwrap()
        .starts_with("asset:"));
    assert_eq!(
        blocks[0]["text"],
        format!("[数据库](page:{})", child["id"].as_str().unwrap())
    );
    assert_eq!(blocks[1]["textColor"], "green");
    let exported = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let other = create(&mut s, "备份恢复");
    s.dispatch(
        "import",
        json!({"workspaceId":other,"name":"备份.zip","data":exported["data"]}),
    )
    .unwrap();
    let restored = s.dispatch("pages", json!({"workspaceId":other})).unwrap();
    let root = restored
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "后端开发")
        .unwrap();
    for asset in [
        root["icon"].as_str().unwrap(),
        root["blocks"][0]["linkIcon"].as_str().unwrap(),
    ] {
        s.dispatch(
            "asset",
            json!({"workspaceId":other,"name":asset.strip_prefix("asset:").unwrap()}),
        )
        .unwrap();
    }
}

#[test]
fn repeated_html_page_links_keep_their_individual_styles() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "重复页面条目");
    let parent = r#"<article class="page"><header><h1 class="page-title">父</h1></header><div class="page-body"><p>参考 <a href="父/子.html">子</a></p><figure class="link-to-page block-color-red"><a href="父/子.html"><span class="icon">🔴</span>子</a></figure><figure class="link-to-page block-color-green"><a href="父/子.html"><span class="icon">🟢</span>子</a></figure></div></article>"#;
    let child = r#"<article class="page"><header><h1 class="page-title">子</h1></header><div class="page-body"><p>原文</p></div></article>"#;
    s.dispatch("import",json!({"workspaceId":w,"name":"重复.zip","data":base64::engine::general_purpose::STANDARD.encode(zip_files(&[("父.html",parent),("父/子.html",child)]))})).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    let parent = pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["title"] == "父")
        .unwrap();
    assert!(parent["blocks"][0]["textColor"].is_null());
    assert_eq!(parent["blocks"][1]["textColor"], "red");
    assert_eq!(parent["blocks"][1]["linkIcon"], "🔴");
    assert_eq!(parent["blocks"][2]["textColor"], "green");
    assert_eq!(parent["blocks"][2]["linkIcon"], "🟢");
}

#[test]
fn pasted_markdown_export_preserves_headings_lists_quotes_and_fence_content() {
    use base64::Engine;
    use std::io::Read;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "文档粘贴导出");
    s.dispatch("save",json!({"workspaceId":w,"page":{"title":"粘贴文档","blocks":[{"id":"heading","type":"h6","text":"六级标题"},{"id":"number","type":"number","order":3,"text":"第三项"},{"id":"nested","type":"bullet","depth":1,"text":"子项"},{"id":"quote","type":"quote","quoteDepth":2,"text":"第一行\n第二行"},{"id":"code","type":"code","language":"text","text":"first\n```\nsecond\n````\nlast"}]}})).unwrap();
    let exported = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(exported["data"].as_str().unwrap())
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut markdown = String::new();
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).unwrap();
        if file.name().ends_with(".md") {
            file.read_to_string(&mut markdown).unwrap();
            break;
        }
    }
    assert!(markdown.contains("###### 六级标题"));
    assert!(markdown.contains("3. 第三项"));
    assert!(markdown.contains("    - 子项"));
    assert!(markdown.contains("> > 第一行\n> > 第二行"));
    assert!(markdown.contains("`````text\nfirst\n```\nsecond\n````\nlast\n`````"));
}

#[test]
fn markdown_file_roundtrip_keeps_long_and_tilde_fences_and_following_text() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "围栏往返");
    for (index, source) in [
        "###### 六级标题\n\n`````text\nfirst\n```\nsecond\n````\nlast\n`````\n\n后续正文",
        "~~~lang`info\nfirst\n```\n~~~ not closing\nlast\n~~~\n\n后续正文",
        "   ````text\n   first\n   ```\n   last\n   ````\n\n后续正文",
    ]
    .iter()
    .enumerate()
    {
        s.dispatch("import",json!({"workspaceId":w,"name":format!("fence{index}.md"),"data":base64::engine::general_purpose::STANDARD.encode(source)})).unwrap();
        let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
        let page = pages
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["title"] == format!("fence{index}"))
            .unwrap();
        let blocks = page["blocks"].as_array().unwrap();
        let code = blocks.iter().find(|b| b["type"] == "code").unwrap();
        let expected = [
            "first\n```\nsecond\n````\nlast",
            "first\n```\n~~~ not closing\nlast",
            "first\n```\nlast",
        ][index];
        assert_eq!(code["text"], expected);
        assert_eq!(blocks.last().unwrap()["text"], "后续正文");
        if index == 0 {
            assert_eq!(blocks[0]["type"], "h6");
        }
        let exported = s.dispatch("export", json!({"workspaceId":w})).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(exported["data"].as_str().unwrap())
            .unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut exported_markdown = String::new();
        for file_index in 0..zip.len() {
            let mut file = zip.by_index(file_index).unwrap();
            if file.name() == format!("markdown/{}.md", page["id"].as_str().unwrap()) {
                use std::io::Read;
                file.read_to_string(&mut exported_markdown).unwrap();
                break;
            }
        }
        assert!(!exported_markdown.is_empty());
        let other = create(&mut s, "文本恢复");
        s.dispatch("import",json!({"workspaceId":other,"name":"restored.md","data":base64::engine::general_purpose::STANDARD.encode(exported_markdown)})).unwrap();
        let restored = s.dispatch("pages", json!({"workspaceId":other})).unwrap();
        assert_eq!(
            restored[0]["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["type"] == "code")
                .unwrap()["text"],
            expected
        );
        assert_eq!(
            restored[0]["blocks"].as_array().unwrap().last().unwrap()["text"],
            "后续正文"
        );
    }
}

#[test]
fn complex_markdown_import_keeps_original_container_order() {
    use base64::Engine;
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "复杂文档");
    for (index, source) in [
        "- 父项\n  > 引用正文\n  >\n  > - 引用子项\n\n- 第二项",
        "> - 父项\n>   - 子项",
        "1000. 父项\n      - 子项",
        "> 第一行\n>\n>> 二级",
        "[地址][ref]\n\n[ref]: https://example.com",
    ]
    .iter()
    .enumerate()
    {
        s.dispatch("import",json!({"workspaceId":w,"name":format!("complex{index}.md"),"data":base64::engine::general_purpose::STANDARD.encode(source)})).unwrap();
        let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
        let page = pages
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["title"] == format!("complex{index}"))
            .unwrap();
        assert_eq!(page["blocks"].as_array().unwrap().len(), 1);
        assert_eq!(page["blocks"][0]["markdownDocument"], true);
        assert_eq!(page["blocks"][0]["text"], *source);
    }
}

fn named_view_database() -> serde_json::Value {
    json!({"columns":[{"id":"name","name":"名称","type":"text"},{"id":"status","name":"状态","type":"status","options":["进行中","已完成"]},{"id":"amount","name":"金额","type":"number"}],"rows":[{"id":"record","values":{"name":"原记录","status":"进行中","amount":"0"}}],"views":[{"id":"view","name":"进行中","type":"table","filter":{"conjunction":"and","rules":[{"id":"status-rule","propertyId":"status","operator":"eq","value":"进行中"}]},"sorts":[{"propertyId":"amount","direction":"desc"}],"hiddenColumnIds":["amount"],"groupColumnId":"status","dateColumnId":""}]})
}
#[test]
fn named_views_reject_malformed_configuration_without_partial_save() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "视图校验");
    let mut cases = vec![];
    for (key, value) in [
        ("type", json!("script")),
        ("name", json!("")),
        ("sorts", json!([{"propertyId":"missing","direction":"asc"}])),
        ("hiddenColumnIds", json!(["name"])),
        ("groupColumnId", json!("amount")),
    ] {
        let mut db = named_view_database();
        db["views"][0][key] = value;
        cases.push(db);
    }
    for (key, value) in [("propertyId", json!("missing")), ("operator", json!("gt"))] {
        let mut db = named_view_database();
        db["views"][0]["filter"]["rules"][0][key] = value;
        cases.push(db);
    }
    let mut db = named_view_database();
    let duplicate = db["views"][0].clone();
    db["views"].as_array_mut().unwrap().push(duplicate);
    cases.push(db);
    let mut db = named_view_database();
    db["views"][0]["filter"]["rules"][0] =
        json!({"id":"number","propertyId":"amount","operator":"gt","value":"文字"});
    cases.push(db);
    for db in cases {
        assert!(s
            .dispatch(
                "save",
                json!({"workspaceId":w,"page":{"title":"应拒绝","blocks":[],"database":db}})
            )
            .is_err());
    }
    assert_eq!(
        s.dispatch("pages", json!({"workspaceId":w}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
}
#[test]
fn named_views_survive_restart_and_structured_backup() {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::new(d.path()).unwrap();
    let w = create(&mut s, "视图持久化");
    let db = named_view_database();
    s.dispatch(
        "save",
        json!({"workspaceId":w,"page":{"title":"视图页面","blocks":[],"database":db}}),
    )
    .unwrap();
    drop(s);
    let mut s = Store::new(d.path()).unwrap();
    let pages = s.dispatch("pages", json!({"workspaceId":w})).unwrap();
    assert_eq!(pages[0]["database"]["views"], db["views"]);
    let exported = s.dispatch("export", json!({"workspaceId":w})).unwrap();
    let other = create(&mut s, "视图恢复");
    s.dispatch(
        "import",
        json!({"workspaceId":other,"name":"视图.zip","data":exported["data"]}),
    )
    .unwrap();
    let restored = s.dispatch("pages", json!({"workspaceId":other})).unwrap();
    assert_eq!(restored[0]["database"]["views"], db["views"]);
    assert_eq!(restored[0]["database"]["rows"].as_array().unwrap().len(), 1);
}
