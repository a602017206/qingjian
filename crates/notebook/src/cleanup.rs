use crate::{err, now, text, uid, valid_id, write_json, Result, Store};
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    time::UNIX_EPOCH,
};
fn requested(q: &Value, pages: &[Value], all: bool) -> Result<HashSet<String>> {
    let mut ids = if all {
        pages
            .iter()
            .filter(|p| p["deleted"] == true)
            .map(|p| text(p, "id").to_owned())
            .collect()
    } else {
        let values = q["ids"].as_array().ok_or("请选择页面")?;
        if values.is_empty() {
            return Err("请选择页面".into());
        }
        let mut ids = HashSet::new();
        for value in values {
            let id = value.as_str().ok_or("页面标识无效")?;
            if !valid_id(id) || !pages.iter().any(|p| text(p, "id") == id) {
                return Err("页面不存在或标识无效".into());
            }
            ids.insert(id.to_owned());
        }
        ids
    };
    loop {
        let before = ids.len();
        for p in pages {
            if ids.contains(text(p, "parentId")) {
                ids.insert(text(p, "id").to_owned());
            }
        }
        if ids.len() == before {
            break;
        }
    }
    Ok(ids)
}
fn revisions(pages: &[Value]) -> Value {
    Value::Object(
        pages
            .iter()
            .map(|p| (text(p, "id").to_owned(), p["revision"].clone()))
            .collect(),
    )
}
pub(crate) fn trash_many(dir: &Path, q: &Value) -> Result<Value> {
    let pages = Store::pages_at(dir)?;
    let mut ids = requested(q, &pages, false)?;
    let deleted = q["deleted"].as_bool().ok_or("回收操作无效")?;
    if !deleted {
        loop {
            let before = ids.len();
            for p in &pages {
                if ids.contains(text(p, "id")) && !text(p, "parentId").is_empty() {
                    ids.insert(text(p, "parentId").to_owned());
                }
            }
            if before == ids.len() {
                break;
            }
        }
    }
    for p in &pages {
        if ids.contains(text(p, "id")) && q["revisions"][text(p, "id")] != p["revision"] {
            return Err("页面已变化，请刷新后重新选择".into());
        }
    }
    ids.retain(|id| {
        pages
            .iter()
            .any(|p| text(p, "id") == id && (p["deleted"] == true) != deleted)
    });
    let stage = dir.join(format!(".trash-{}", uid()));
    fs::create_dir(&stage).map_err(err)?;
    let mut published = vec![];
    let result: Result<()> = (|| {
        for p in &pages {
            if !ids.contains(text(p, "id")) {
                continue;
            }
            let id = text(p, "id");
            fs::copy(
                dir.join("pages").join(format!("{id}.json")),
                stage.join(format!("{id}.old")),
            )
            .map_err(err)?;
            let mut next = p.clone();
            next["deleted"] = json!(deleted);
            next["updatedAt"] = json!(now());
            next["revision"] = json!(p["revision"].as_u64().unwrap_or(0) + 1);
            write_json(&stage.join(format!("{id}.new")), &next)?;
        }
        for p in &pages {
            let id = text(p, "id");
            if ids.contains(id) {
                fs::rename(
                    stage.join(format!("{id}.new")),
                    dir.join("pages").join(format!("{id}.json")),
                )
                .map_err(err)?;
                published.push(id.to_owned());
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_errors = vec![];
        for id in published {
            if let Err(e) = fs::rename(
                stage.join(format!("{id}.old")),
                dir.join("pages").join(format!("{id}.json")),
            ) {
                rollback_errors.push(e.to_string());
            }
        }
        if !rollback_errors.is_empty() {
            return Err(format!(
                "批量回收中断，恢复未完成，请保留临时目录：{}；{}",
                stage.display(),
                rollback_errors.join("；")
            ));
        }
        let _ = fs::remove_dir_all(stage);
        return Err(error);
    }
    fs::remove_dir_all(stage).map_err(|e| format!("页面已回收，但临时文件清理失败：{e}"))?;
    Ok(json!({"count":ids.len()}))
}
pub(crate) fn asset_refs(page: &Value) -> HashSet<String> {
    let body = page.to_string();
    Regex::new(r"asset:([A-Za-z0-9_-]+(?:\.[A-Za-z0-9]+)?)")
        .unwrap()
        .captures_iter(&body)
        .map(|c| c[1].to_owned())
        .collect()
}
pub(crate) fn preview(dir: &Path, q: &Value) -> Result<Value> {
    let pages = Store::pages_at(dir)?;
    let selected = requested(q, &pages, q["all"] == true)?;
    if pages
        .iter()
        .any(|p| selected.contains(text(p, "id")) && p["deleted"] != true)
    {
        return Err("只能永久删除回收站中的页面，请先处理仍在使用的子页面".into());
    }
    let retained: HashSet<_> = pages
        .iter()
        .filter(|p| !selected.contains(text(p, "id")))
        .flat_map(asset_refs)
        .collect();
    let candidates: HashSet<_> = pages
        .iter()
        .filter(|p| selected.contains(text(p, "id")))
        .flat_map(asset_refs)
        .filter(|name| !retained.contains(name))
        .collect();
    let mut inventory = vec![];
    let mut assets = vec![];
    let mut bytes = 0u64;
    for entry in fs::read_dir(dir.join("assets")).map_err(err)? {
        let entry = entry.map_err(err)?;
        let m = fs::symlink_metadata(entry.path()).map_err(err)?;
        if !m.file_type().is_file() {
            return Err("附件目录包含特殊文件，清理已停止".into());
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let modified = m
            .modified()
            .map_err(err)?
            .duration_since(UNIX_EPOCH)
            .map_err(err)?
            .as_nanos()
            .to_string();
        inventory.push(json!({"name":name,"bytes":m.len(),"modified":modified}));
        if candidates.contains(&name) {
            bytes += m.len();
            assets.push(name);
        }
    }
    inventory.sort_by(|a, b| text(a, "name").cmp(text(b, "name")));
    assets.sort();
    let mut ids: Vec<_> = selected.into_iter().collect();
    ids.sort();
    for id in &ids {
        bytes += fs::metadata(dir.join("pages").join(format!("{id}.json")))
            .map_err(err)?
            .len();
    }
    Ok(
        json!({"ids":ids,"assets":assets,"bytes":bytes,"revisions":revisions(&pages),"inventory":inventory}),
    )
}
pub(crate) fn purge(dir: &Path, q: &Value) -> Result<Value> {
    if q["confirmed"] != true {
        return Err("永久删除需要确认预览结果".into());
    }
    let expected = &q["preview"];
    let current = preview(dir, &json!({"ids":expected["ids"]}))?;
    if current != *expected {
        return Err("页面或附件已变化，未执行删除，请重新预览".into());
    }
    let mut removed_pages = 0;
    let mut removed_assets = 0;
    let mut bytes = 0u64;
    let mut errors = vec![];
    let pages = Store::pages_at(dir)?;
    let parents: HashMap<_, _> = pages
        .iter()
        .map(|p| (text(p, "id"), text(p, "parentId")))
        .collect();
    let depth = |id: &str| {
        let mut n = 0;
        let mut cursor = id;
        while let Some(parent) = parents.get(cursor) {
            if parent.is_empty() {
                break;
            }
            n += 1;
            cursor = parent;
        }
        n
    };
    let mut ids: Vec<String> = current["ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    ids.sort_by(|a, b| depth(b).cmp(&depth(a)).then(a.cmp(b)));
    let assets: Vec<String> = current["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    for (key, folder, names) in [("ids", "pages", ids), ("assets", "assets", assets)] {
        for name in names {
            let path = dir.join(folder).join(if key == "ids" {
                format!("{name}.json")
            } else {
                name.to_owned()
            });
            let len = match fs::metadata(&path) {
                Ok(meta) => meta.len(),
                Err(error) => {
                    errors.push(format!("清理 {name} 前检查失败：{error}"));
                    break;
                }
            };
            match fs::remove_file(path) {
                Ok(()) => {
                    bytes += len;
                    if key == "ids" {
                        removed_pages += 1;
                    } else {
                        removed_assets += 1;
                    }
                }
                Err(e) => {
                    errors.push(format!("清理 {name} 失败：{e}"));
                    break;
                }
            }
        }
        if !errors.is_empty() {
            break;
        }
    }
    Ok(json!({"pages":removed_pages,"assets":removed_assets,"bytes":bytes,"errors":errors}))
}
