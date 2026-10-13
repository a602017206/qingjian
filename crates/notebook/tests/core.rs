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
