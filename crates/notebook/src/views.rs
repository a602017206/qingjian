use crate::{text, valid_id, Result};
use serde_json::Value;
use std::collections::HashSet;

fn operators(kind: &str) -> Vec<&'static str> {
    let mut values = match kind {
        "checkbox" => return vec!["eq", "ne"],
        "multiselect" | "person" | "relation" | "files" => vec!["contains", "not_contains"],
        "number" | "uniqueId" => vec!["eq", "ne", "gt", "gte", "lt", "lte"],
        "date" | "createdTime" | "editedTime" => {
            vec!["eq", "ne", "before", "after", "on_or_before", "on_or_after"]
        }
        "select" | "status" => vec!["eq", "ne"],
        "formula" | "rollup" => vec![
            "eq",
            "ne",
            "contains",
            "not_contains",
            "gt",
            "gte",
            "lt",
            "lte",
        ],
        _ => vec![
            "eq",
            "ne",
            "contains",
            "not_contains",
            "starts_with",
            "ends_with",
        ],
    };
    values.extend(["empty", "not_empty"]);
    values
}
fn filter(
    node: &Value,
    columns: &[Value],
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<String>,
) -> Result<()> {
    *count += 1;
    if depth > 3 || *count > 64 || !["and", "or"].contains(&text(node, "conjunction")) {
        return Err("视图条件数量或嵌套深度无效".into());
    }
    let rules = node["rules"].as_array().ok_or("视图筛选格式无效")?;
    for rule in rules {
        if !valid_id(text(rule, "id")) || !ids.insert(text(rule, "id").to_owned()) {
            return Err("视图条件标识无效或重复".into());
        }
        if rule.get("rules").is_some() {
            filter(rule, columns, depth + 1, count, ids)?;
            continue;
        }
        *count += 1;
        if *count > 64 {
            return Err("视图条件最多六十四项".into());
        }
        let column = columns
            .iter()
            .find(|c| c["id"] == rule["propertyId"])
            .ok_or("视图筛选属性不存在")?;
        let operator = text(rule, "operator");
        let value = rule["value"].as_str().ok_or("视图筛选值必须是文本")?;
        if value.chars().count() > 4096 || !operators(text(column, "type")).contains(&operator) {
            return Err("视图筛选条件无效".into());
        }
        if ["empty", "not_empty"].contains(&operator) {
            continue;
        }
        let kind = text(column, "type");
        if (["number", "uniqueId"].contains(&kind)
            || ["gt", "gte", "lt", "lte"].contains(&operator))
            && !value.trim().parse::<f64>().is_ok_and(f64::is_finite)
        {
            return Err("视图筛选需要有效数字".into());
        }
        if (["date", "createdTime", "editedTime"].contains(&kind)
            || ["before", "after", "on_or_before", "on_or_after"].contains(&operator))
            && (value.len() != 10 || chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_err())
        {
            return Err("视图筛选日期无效".into());
        }
        if kind == "checkbox" && !["true", "false"].contains(&value) {
            return Err("视图复选框条件无效".into());
        }
    }
    Ok(())
}
pub(crate) fn validate(db: &Value) -> Result<()> {
    let Some(value) = db.get("views") else {
        return Ok(());
    };
    let views = value.as_array().ok_or("数据库视图必须是列表")?;
    if views.is_empty() || views.len() > 32 {
        return Err("数据库视图数量须为一至三十二个".into());
    }
    let columns = db["columns"].as_array().ok_or("数据库属性格式无效")?;
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for view in views {
        let name = text(view, "name");
        if !valid_id(text(view, "id"))
            || !ids.insert(text(view, "id"))
            || name.trim().is_empty()
            || name.chars().count() > 120
            || !names.insert(name.trim())
            || !["table", "board", "list", "gallery", "calendar"].contains(&text(view, "type"))
        {
            return Err("数据库视图名称、标识或类型无效".into());
        }
        filter(&view["filter"], columns, 0, &mut 0, &mut HashSet::new())?;
        let sorts = view["sorts"].as_array().ok_or("视图排序必须是列表")?;
        let mut sort_ids = HashSet::new();
        if sorts.len() > 8 {
            return Err("视图最多设置八层排序".into());
        }
        for sort in sorts {
            if !columns.iter().any(|c| c["id"] == sort["propertyId"])
                || !sort_ids.insert(text(sort, "propertyId"))
                || !["asc", "desc"].contains(&text(sort, "direction"))
            {
                return Err("视图排序属性或方向无效".into());
            }
        }
        let hidden = view["hiddenColumnIds"]
            .as_array()
            .ok_or("视图属性显示格式无效")?;
        let mut hidden_ids = HashSet::new();
        for id in hidden {
            if !id.is_string()
                || columns.first().is_some_and(|c| c["id"] == *id)
                || !columns.iter().any(|c| c["id"] == *id)
                || !hidden_ids.insert(id.as_str().unwrap())
            {
                return Err("视图隐藏属性无效，名称属性必须保留".into());
            }
        }
        for (key, kinds) in [
            ("groupColumnId", vec!["select", "status"]),
            ("dateColumnId", vec!["date"]),
        ] {
            if let Some(id) = view.get(key) {
                if !id.is_string()
                    || !text(view, key).is_empty()
                        && !columns
                            .iter()
                            .any(|c| c["id"] == *id && kinds.contains(&text(c, "type")))
                {
                    return Err("视图分组或日期属性无效".into());
                }
            }
        }
    }
    Ok(())
}
