use notebook_core::Store;
use serde_json::json;
#[test]
fn structured_dates_validate_without_migrating_legacy_values() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::new(root.path()).unwrap();
    let id = store
        .dispatch("createWorkspace", json!({"name":"匿名日期"}))
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let page=store.dispatch("save",json!({"workspaceId":id,"page":{"title":"日期数据库","blocks":[],"database":{"columns":[{"id":"name","name":"名称","type":"text"},{"id":"date","name":"日期","type":"date"}],"rows":[{"id":"old","values":{"name":"旧记录","date":"旧导入原文"}},{"id":"range","values":{"date":"2024-02-29"}}]}}})).unwrap();
    for value in [
        json!({"kind":"dateRange","start":"2023-02-29","end":"2023-03-01"}),
        json!({"kind":"dateRange","start":"2026-10-28","end":"2026-10-27"}),
        json!({"kind":"dateRange","start":"2026-10-28T23:00+08:00","end":"2026-10-28T22:00+08:00","timeZone":"Asia/Shanghai"}),
        json!({"kind":"dateRange","start":"2026-10-28T09:00","timeZone":"Asia/Shanghai"}),
        json!({"kind":"dateRange","start":"2026-10-28","end":22}),
    ] {
        let mut draft = page.clone();
        draft["database"]["rows"][1]["values"]["date"] = json!(value.to_string());
        assert!(
            store
                .dispatch("save", json!({"workspaceId":id,"page":draft}))
                .is_err(),
            "必须拒绝无效范围：{value}"
        );
        assert_eq!(
            store.dispatch("pages", json!({"workspaceId":id})).unwrap()[0],
            page
        );
    }
    let mut draft = page.clone();
    let date=json!({"kind":"dateRange","start":"2026-10-28T23:30+08:00","end":"2026-10-29T01:00+08:00","timeZone":"Asia/Shanghai"}).to_string();
    draft["database"]["rows"][1]["values"]["date"] = json!(date);
    let saved = store
        .dispatch("save", json!({"workspaceId":id,"page":draft}))
        .unwrap();
    assert_eq!(saved["database"]["rows"][1]["values"]["date"], date);
    assert_eq!(saved["database"]["rows"][0]["values"]["date"], "旧导入原文");
    let mut draft = saved;
    draft["database"]["rows"][1]["values"]["date"] =
        json!(json!({"kind":"dateRange","start":"2026-10-28T09:30Z","timeZone":"UTC"}).to_string());
    assert!(
        store
            .dispatch("save", json!({"workspaceId":id,"page":draft}))
            .is_ok(),
        "分钟精度UTC值也必须兼容"
    );
    let mut legacy = store.dispatch("pages", json!({"workspaceId":id})).unwrap()[0].clone();
    for raw in [
        "{旧导入原文",
        r#"{"legacy":"日期原始对象"}"#,
        r#"{"start":"旧日期原文","source":true}"#,
    ] {
        legacy["database"]["rows"][0]["values"]["date"] = json!(raw);
        legacy = store
            .dispatch("save", json!({"workspaceId":id,"page":legacy}))
            .expect("旧客户端允许的原值不得阻止整个工作区加载");
        assert_eq!(
            store.dispatch("pages", json!({"workspaceId":id})).unwrap()[0]["database"]["rows"][0]
                ["values"]["date"],
            raw
        );
    }
}
