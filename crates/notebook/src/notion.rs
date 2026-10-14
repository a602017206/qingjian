use crate::{err, text, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
type CsvData = (Vec<String>, Vec<Vec<String>>);
fn csv_data(data: &[u8]) -> Result<CsvData> {
    let source = std::str::from_utf8(data)
        .map_err(|_| "导出表格字符编码无效")?
        .trim_start_matches('\u{feff}');
    let mut reader = csv::Reader::from_reader(source.as_bytes());
    let headers = reader
        .headers()
        .map_err(err)?
        .iter()
        .map(str::to_owned)
        .collect();
    let rows = reader
        .records()
        .map(|r| {
            r.map(|r| r.iter().map(str::to_owned).collect())
                .map_err(err)
        })
        .collect::<Result<Vec<_>>>()?;
    Ok((headers, rows))
}
fn matched_rows(view: &CsvData, full: &CsvData) -> Option<Vec<usize>> {
    if view.0.is_empty()
        || view.0.first() != full.0.first()
        || view.0.iter().collect::<HashSet<_>>().len() != view.0.len()
        || full.0.iter().collect::<HashSet<_>>().len() != full.0.len()
    {
        return None;
    }
    let positions = view
        .0
        .iter()
        .map(|h| full.0.iter().position(|v| v == h))
        .collect::<Option<Vec<_>>>()?;
    let mut available: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, row) in full.1.iter().enumerate().rev() {
        available
            .entry(positions.iter().map(|&index| row[index].clone()).collect())
            .or_default()
            .push(i);
    }
    let mut order = vec![];
    for row in &view.1 {
        order.push(available.get_mut(row)?.pop()?);
    }
    Some(order)
}
pub(crate) fn variants(files: &[(String, Vec<u8>)]) -> Result<HashMap<String, String>> {
    let index: HashMap<&str, &[u8]> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let pattern = Regex::new(r"(?i)\s+[0-9a-f]{32}_all\.csv$").unwrap();
    let mut pairs = HashMap::new();
    for (full, data) in files {
        if !pattern.is_match(full) {
            continue;
        }
        let base = format!("{}.csv", full.strip_suffix("_all.csv").unwrap_or(""));
        if let Some(view) = index.get(base.as_str()) {
            if matched_rows(&csv_data(view)?, &csv_data(data)?).is_some() {
                pairs.insert(base, full.clone());
            }
        }
    }
    Ok(pairs)
}
pub(crate) fn apply_view(
    db: &mut Value,
    name: &str,
    pairs: &HashMap<String, String>,
    files: &[(String, Vec<u8>)],
) -> Result<()> {
    let Some((base, _)) = pairs.iter().find(|(_, full)| full.as_str() == name) else {
        return Ok(());
    };
    let view = csv_data(&files.iter().find(|(n, _)| n == base).unwrap().1)?;
    let full = csv_data(&files.iter().find(|(n, _)| n == name).unwrap().1)?;
    let order = matched_rows(&view, &full).ok_or("展示表格和完整表格无法核对")?;
    let columns = db["columns"].as_array_mut().unwrap();
    columns.sort_by_key(|c| {
        view.0
            .iter()
            .position(|h| h == text(c, "name"))
            .unwrap_or(view.0.len())
    });
    let rows = db["rows"].as_array_mut().unwrap();
    let original = rows.clone();
    let used: HashSet<_> = order.iter().copied().collect();
    *rows = order
        .iter()
        .map(|&i| original[i].clone())
        .chain(
            original
                .iter()
                .enumerate()
                .filter(|(i, _)| !used.contains(i))
                .map(|(_, r)| r.clone()),
        )
        .collect();
    db["importedViews"] =
        json!([{"source":base,"columns":view.0},{"source":name,"columns":full.0}]);
    Ok(())
}
fn stem_path(name: &str, clean: bool) -> String {
    let path = Path::new(name);
    let mut stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let suffix = Regex::new(r"(?i)\s+[0-9a-f]{32}(?:_all)?$").unwrap();
    if clean {
        stem = suffix.replace(&stem, "").into_owned();
    } else if Regex::new(r"(?i)\s+[0-9a-f]{32}_all$")
        .unwrap()
        .is_match(&stem)
    {
        stem = stem.strip_suffix("_all").unwrap_or(&stem).to_owned();
    }
    path.parent()
        .unwrap_or(Path::new(""))
        .join(stem)
        .to_string_lossy()
        .replace('\\', "/")
}
pub(crate) fn assign_parents(
    pages: &mut [Value],
    paths: &HashMap<String, String>,
    warnings: &mut Vec<String>,
) {
    let mut exact: HashMap<String, HashSet<String>> = HashMap::new();
    let mut aliases: HashMap<String, HashSet<String>> = HashMap::new();
    for (name, id) in paths {
        exact
            .entry(stem_path(name, false))
            .or_default()
            .insert(id.clone());
        aliases
            .entry(stem_path(name, true))
            .or_default()
            .insert(id.clone());
    }
    for page in pages {
        let source = text(page, "importSource").to_owned();
        let mut directory = Path::new(&source).parent();
        while let Some(path) = directory {
            if path.as_os_str().is_empty() {
                break;
            }
            let key = path.to_string_lossy().replace('\\', "/");
            if let Some(ids) = exact.get(&key).or_else(|| aliases.get(&key)) {
                if ids.len() == 1 {
                    let id = ids.iter().next().unwrap();
                    if id != text(page, "id") {
                        page["parentId"] = json!(id);
                    }
                    break;
                }
                warnings.push(format!(
                    "目录 {key} 有多个可能的父页面，未猜测子页面归属：{source}"
                ));
                break;
            }
            directory = path.parent();
        }
    }
}
pub(crate) fn link_records(
    pages: &mut [Value],
    files: &[(String, Vec<u8>)],
    warnings: &mut Vec<String>,
) {
    let files: HashMap<&str, &[u8]> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let mut candidates: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (child_index, child) in pages.iter().enumerate() {
        if child["database"].is_object() {
            continue;
        }
        let Some((parent_index, parent)) = pages
            .iter()
            .enumerate()
            .find(|(_, p)| text(p, "id") == text(child, "parentId") && p["database"].is_object())
        else {
            continue;
        };
        let Some(source) = files.get(text(child, "importSource")) else {
            continue;
        };
        let Ok(body) = std::str::from_utf8(source) else {
            continue;
        };
        let columns = parent["database"]["columns"].as_array().unwrap();
        let rows = parent["database"]["rows"].as_array().unwrap();
        if rows.iter().any(|r| text(r, "pageId") == text(child, "id")) {
            continue;
        }
        let mut properties = HashMap::new();
        let lines: Vec<_> = body.lines().collect();
        let start = if lines.first().map(|l| l.starts_with("# ")).unwrap_or(false) {
            1
        } else {
            0
        };
        for line in lines[start..]
            .iter()
            .skip_while(|l| l.trim().is_empty())
            .take_while(|l| !l.trim().is_empty())
        {
            if let Some((key, value)) = line.split_once(": ") {
                if columns.iter().any(|c| text(c, "name") == key) {
                    properties.insert(key, value.trim());
                }
            }
        }
        let matches: Vec<_> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                (child.get("importProperties").is_none()
                    || (text(row, "sourceId").is_empty() && text(row, "sourceHref").is_empty()))
                    && text(&row["sourceValues"], text(&columns[0], "id")) == text(child, "title")
                    && child["importProperties"]
                        .as_array()
                        .map(|props| {
                            props.iter().all(|p| {
                                columns
                                    .iter()
                                    .find(|c| text(c, "name") == text(p, "name"))
                                    .map(|c| {
                                        crate::html_import::property_matches(
                                            text(&row["sourceValues"], text(c, "id")),
                                            p,
                                        )
                                    })
                                    .unwrap_or(true)
                            })
                        })
                        .unwrap_or(true)
                    && columns.iter().all(|c| {
                        properties
                            .get(text(c, "name"))
                            .map(|value| text(&row["sourceValues"], text(c, "id")).trim() == *value)
                            .unwrap_or(true)
                    })
            })
            .map(|(i, _)| i)
            .collect();
        if matches.len() == 1 {
            candidates
                .entry((parent_index, matches[0]))
                .or_default()
                .push(child_index);
        } else {
            warnings.push(format!(
                "记录与子页面无法唯一对应，保留独立内容：{}",
                text(child, "title")
            ));
        }
    }
    for ((parent, row), children) in candidates {
        if children.len() == 1 {
            pages[parent]["database"]["rows"][row]["pageId"] = pages[children[0]]["id"].clone();
        } else {
            warnings.push("多个子页面对应同一条记录，未合并正文。".into());
        }
    }
}
