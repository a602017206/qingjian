use crate::{text, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

const META: [(&str, &str); 4] = [
    ("createdAt", "createdTime"),
    ("createdBy", "createdBy"),
    ("updatedAt", "editedTime"),
    ("updatedBy", "editedBy"),
];

pub(crate) fn validate_value(kind: &str, raw: &str) -> Result<()> {
    if raw.trim().is_empty() {
        return Ok(());
    }
    match kind {
        "person" => {
            serde_json::from_str::<Vec<String>>(raw)
                .map_err(|_| "人员必须是文本名单".to_owned())?;
        }
        "files" => {
            let files: Vec<Value> = serde_json::from_str(raw).map_err(|_| "文件和媒体格式无效")?;
            for file in files {
                if !file.is_object() || !file["name"].is_string() {
                    return Err("文件名称格式无效".into());
                }
                let asset = text(&file, "asset");
                let url = text(&file, "url");
                if !asset.is_empty() {
                    let name = asset.strip_prefix("asset:").ok_or("附件引用格式无效")?;
                    if name.is_empty()
                        || name.len() > 100
                        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
                        || name.starts_with('.')
                        || name.contains("..")
                    {
                        return Err("附件引用路径无效".into());
                    }
                }
                if !url.is_empty() && !(url.starts_with("https://") || url.starts_with("http://")) {
                    return Err("媒体链接必须是网页地址".into());
                }
                for key in ["asset", "url"] {
                    if file.get(key).is_some_and(|v| !v.is_string()) {
                        return Err("媒体引用必须是文本".into());
                    }
                }
            }
        }
        "location" => {
            let location: Value = serde_json::from_str(raw).map_err(|_| "位置格式无效")?;
            if !location.is_object() || !location["address"].is_string() {
                return Err("位置地址必须是文本".into());
            }
            if location.get("latitude").is_some() != location.get("longitude").is_some() {
                return Err("位置经纬度必须成对填写".into());
            }
            for (key, limit) in [("latitude", 90.), ("longitude", 180.)] {
                if let Some(value) = location.get(key) {
                    if !value
                        .as_f64()
                        .is_some_and(|v| v.is_finite() && v.abs() <= limit)
                    {
                        return Err("位置经纬度超出有效范围".into());
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn validate_database(db: &Value) -> Result<()> {
    let columns = db["columns"].as_array().ok_or("数据库属性格式无效")?;
    let rows = db["rows"].as_array().ok_or("数据库记录格式无效")?;
    for column in columns {
        for key in ["buttonLabel", "uniqueIdPrefix"] {
            if column.get(key).is_some_and(|v| !v.is_string()) {
                return Err("按钮名称和编号前缀必须是文本".into());
            }
        }
        if let Some(action) = column.get("buttonAction").filter(|v| !v.is_null()) {
            let target = columns
                .iter()
                .find(|c| text(c, "id") == text(action, "propertyId"));
            if !action.is_object()
                || text(action, "kind") != "set"
                || !action["value"].is_string()
                || target.is_none()
            {
                return Err("按钮操作必须为已有属性赋值".into());
            }
            let target = target.unwrap();
            if ![
                "text",
                "number",
                "select",
                "multiselect",
                "status",
                "date",
                "person",
                "checkbox",
                "url",
                "email",
                "phone",
            ]
            .contains(&text(target, "type"))
                || text(target, "id") == text(&columns[0], "id")
                || text(target, "id") == text(column, "id")
            {
                return Err("按钮只能修改支持的可编辑属性，不能修改名称或自身".into());
            }
            validate_value(text(target, "type"), text(action, "value"))?;
        }
    }
    let mut sequences = HashSet::new();
    for row in rows {
        for (key, _) in META {
            if row.get(key).is_some_and(|v| !v.is_string()) {
                return Err("记录时间和作者必须是文本".into());
            }
        }
        if let Some(sequence) = row.get("sequence") {
            let n = sequence
                .as_u64()
                .filter(|n| *n > 0)
                .ok_or("记录编号必须是正整数")?;
            if !sequences.insert(n) {
                return Err("记录编号重复".into());
            }
        }
        for column in columns {
            validate_value(
                text(column, "type"),
                text(&row["values"], text(column, "id")),
            )?;
        }
    }
    if db
        .get("nextSequence")
        .is_some_and(|v| v.as_u64().filter(|n| *n > 0).is_none())
    {
        return Err("下一个记录编号必须是正整数".into());
    }
    Ok(())
}

pub(crate) fn stamp(page: &mut Value, old: Option<&Value>, local: bool, time: &str) -> Result<()> {
    if let Some(previous) = old {
        page["createdBy"] = json!(text(previous, "createdBy"));
    } else if local || !page["createdBy"].is_string() {
        page["createdBy"] = json!(if local { "本机用户" } else { "" });
    }
    if local {
        page["updatedBy"] = json!("本机用户");
    } else if !page["updatedBy"].is_string() {
        page["updatedBy"] = json!("");
    }
    let page_created = text(page, "createdAt").to_owned();
    let Some(db) = page.get_mut("database").filter(|v| v.is_object()) else {
        return Ok(());
    };
    let columns = db["columns"].as_array().cloned().unwrap_or_default();
    let old_db = old.map(|p| &p["database"]);
    let previous: HashMap<_, _> = old_db
        .and_then(|d| d["rows"].as_array())
        .map(|rows| rows.iter().map(|r| (text(r, "id"), r)).collect())
        .unwrap_or_default();
    let rows = db["rows"].as_array().ok_or("数据库记录格式无效")?;
    let mut next = db["nextSequence"]
        .as_u64()
        .unwrap_or(1)
        .max(old_db.and_then(|d| d["nextSequence"].as_u64()).unwrap_or(1));
    for prior in previous.values() {
        if let Some(n) = prior["sequence"].as_u64() {
            next = next.max(n.checked_add(1).ok_or("记录编号已用尽")?);
        }
    }
    for row in rows {
        let sequence = previous
            .get(text(row, "id"))
            .and_then(|r| r["sequence"].as_u64())
            .or_else(|| {
                if !local {
                    row["sequence"].as_u64()
                } else {
                    None
                }
            });
        if let Some(n) = sequence {
            next = next.max(n.checked_add(1).ok_or("记录编号已用尽")?);
        }
    }
    let mut used = HashSet::new();
    for row in db["rows"].as_array_mut().unwrap() {
        let prior = previous.get(text(row, "id")).copied();
        let sequence = prior.and_then(|r| r["sequence"].as_u64()).or_else(|| {
            if !local {
                row["sequence"].as_u64()
            } else {
                None
            }
        });
        let sequence = if let Some(n) = sequence {
            n
        } else {
            let n = next;
            next = next.checked_add(1).ok_or("记录编号已用尽")?;
            n
        };
        if !used.insert(sequence) {
            return Err("记录编号重复".into());
        }
        row["sequence"] = json!(sequence);
        for (key, kind) in META {
            let value = if let Some(prior) = prior {
                if matches!(key, "updatedAt" | "updatedBy")
                    && (changed_values(&prior["values"], &row["values"])
                        || text(prior, "body") != text(row, "body"))
                {
                    if key == "updatedAt" {
                        time.to_owned()
                    } else {
                        "本机用户".into()
                    }
                } else {
                    if let Some(value) = prior.get(key).and_then(Value::as_str) {
                        value.to_owned()
                    } else {
                        let explicit = old_db
                            .and_then(|d| d["columns"].as_array())
                            .and_then(|cs| cs.iter().find(|c| text(c, "type") == kind))
                            .map(|c| text(&prior["values"], text(c, "id")))
                            .unwrap_or("");
                        if !explicit.is_empty() {
                            explicit.into()
                        } else if matches!(key, "createdAt" | "updatedAt") {
                            old.map(|p| text(p, key)).unwrap_or("").into()
                        } else {
                            String::new()
                        }
                    }
                }
            } else {
                let explicit = columns
                    .iter()
                    .find(|c| text(c, "type") == kind)
                    .map(|c| text(&row["values"], text(c, "id")))
                    .unwrap_or("");
                if !explicit.is_empty() && !local {
                    explicit.to_owned()
                } else if !text(row, key).is_empty() && !local {
                    text(row, key).to_owned()
                } else if matches!(key, "createdAt" | "updatedAt") {
                    if local {
                        time.into()
                    } else {
                        page_created.clone()
                    }
                } else if local {
                    "本机用户".into()
                } else {
                    String::new()
                }
            };
            row[key] = json!(value);
        }
    }
    db["nextSequence"] = json!(next);
    Ok(())
}

fn changed_values(a: &Value, b: &Value) -> bool {
    fn meaningful(v: &Value) -> HashMap<&str, &str> {
        v.as_object()
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(k, v)| {
                        v.as_str()
                            .filter(|s| !s.is_empty())
                            .map(|s| (k.as_str(), s))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    meaningful(a) != meaningful(b)
}
