use crate::{atomic, err, now, text, uid, write_json, Result, Store};
use base64::Engine;
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
const MAX: usize = 128 << 20;
fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase()
}
fn clean_title(name: &str) -> String {
    Regex::new(r"(?i)\s+[0-9a-f]{32}$")
        .unwrap()
        .replace(
            &Path::new(name)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy(),
            "",
        )
        .into_owned()
}
pub fn asset_name(name: &str, data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    let prefix = hash[..16]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let mut ext = extension(name);
    if ext.is_empty() {
        ext = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            "png"
        } else if data.starts_with(b"\xff\xd8\xff") {
            "jpg"
        } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            "gif"
        } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
            "webp"
        } else {
            ""
        }
        .into();
    }
    if ext.is_empty() || ext.len() > 12 || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        prefix
    } else {
        format!("{prefix}.{ext}")
    }
}
fn unpack(name: &str, data: &[u8], depth: u8) -> Result<Vec<(String, Vec<u8>)>> {
    if data.len() > MAX {
        return Err("导入文件超过 128 兆字节，请拆分后导入".into());
    };
    if extension(name) != "zip"
        && !data.starts_with(b"PK\x03\x04")
        && !data.starts_with(b"PK\x05\x06")
    {
        return Ok(vec![(
            Path::new(name)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            data.to_vec(),
        )]);
    };
    if depth > 3 {
        return Err("压缩包嵌套层数过多".into());
    };
    let mut z = ZipArchive::new(Cursor::new(data)).map_err(|_| "压缩包无法读取，请重新导出")?;
    if z.len() > 10000 {
        return Err("文件数超过一万，请分批导入".into());
    };
    let backup = {
        match z.by_name("manifest.json") {
            Ok(mut manifest) => {
                if manifest.size() > 8192 {
                    return Err("备份格式标识过大".into());
                }
                let mut bytes = Vec::new();
                manifest.read_to_end(&mut bytes).map_err(err)?;
                serde_json::from_slice::<Value>(&bytes)
                    .ok()
                    .map(|v| v["format"] == "qingjian" && v["version"] == 1)
                    .unwrap_or(false)
            }
            Err(_) => false,
        }
    };
    let mut contains_notes = false;
    for i in 0..z.len() {
        let file = z.by_index(i).map_err(err)?;
        if !file.is_dir()
            && ["md", "markdown", "csv", "html", "htm"].contains(&extension(file.name()).as_str())
        {
            contains_notes = true;
        }
    }
    let expand_nested = !backup && !contains_notes;
    let mut files = vec![];
    let mut total = 0usize;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(err)?;
        let name = f.name().replace('\\', "/");
        if name.starts_with('/') || name.contains([':', '\0']) || name.split('/').any(|p| p == "..")
        {
            return Err("压缩包包含越界路径，导入已停止".into());
        };
        if f.unix_mode()
            .map(|m| m & 0o170000 == 0o120000)
            .unwrap_or(false)
        {
            return Err("压缩包包含符号链接，导入已停止".into());
        };
        if f.is_dir()
            || name.starts_with("__MACOSX/")
            || name.split('/').any(|p| p == ".git" || p == ".DS_Store")
        {
            continue;
        };
        if f.size() > MAX as u64 {
            return Err("压缩内容超过限制".into());
        };
        let mut b = vec![];
        f.by_ref()
            .take((MAX + 1) as u64)
            .read_to_end(&mut b)
            .map_err(err)?;
        total = total.checked_add(b.len()).ok_or("压缩内容超过限制")?;
        if total > MAX {
            return Err("解压内容超过 128 兆字节，请拆分后导入".into());
        };
        if extension(&name) == "zip" && expand_nested {
            for (nested, bytes) in unpack(&name, &b, depth + 1)? {
                files.push((
                    Path::new(&name)
                        .parent()
                        .unwrap_or(Path::new(""))
                        .join(nested)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    bytes,
                ));
            }
        } else {
            files.push((name, b))
        }
    }
    if files.iter().map(|(_, b)| b.len()).sum::<usize>() > MAX || files.len() > 10000 {
        return Err("解压内容总量超过限制".into());
    };
    Ok(files)
}
fn resolve(name: &str, target: &str) -> String {
    let target =
        percent_encoding::percent_decode_str(target.trim_matches(['<', '>'])).decode_utf8_lossy();
    let path = Path::new(name)
        .parent()
        .unwrap_or(Path::new(""))
        .join(target.split('#').next().unwrap_or(""));
    let mut parts = vec![];
    for p in path.components() {
        match p {
            std::path::Component::ParentDir => {
                parts.pop();
            }
            std::path::Component::Normal(p) => parts.push(p.to_string_lossy().into_owned()),
            _ => (),
        }
    }
    parts.join("/")
}
fn rewrite(
    text: &str,
    name: &str,
    pages: &HashMap<String, String>,
    assets: &HashMap<String, String>,
) -> String {
    Regex::new(r"(!?)\[([^\]]*)\]\(([^)\n]+)\)")
        .unwrap()
        .replace_all(text, |c: &regex::Captures| {
            let target = resolve(name, &c[3]);
            if let Some(id) = pages.get(&target) {
                format!("{}[{}](page:{id})", &c[1], &c[2])
            } else if let Some(asset) = assets.get(&target) {
                format!("{}[{}](asset:{asset})", &c[1], &c[2])
            } else {
                c[0].into()
            }
        })
        .into_owned()
}
pub fn import(dir: &Path, name: &str, data: &[u8]) -> Result<Value> {
    Store::pages_at(dir)?;
    let mut files = unpack(name, data, 0)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let backup = files.iter().any(|(n, b)| {
        n == "manifest.json"
            && serde_json::from_slice::<Value>(b)
                .ok()
                .map(|v| v["format"] == "qingjian" && v["version"] == 1)
                .unwrap_or(false)
    });
    let mut seen = HashSet::new();
    let mut page_ids = HashMap::new();
    let mut asset_ids = HashMap::new();
    let mut assets = HashMap::new();
    let mut original_ids = HashMap::new();
    let mut warnings = vec![];
    if !backup {
        warnings.push("评论、成员权限、公式逻辑和关联汇总无法从导出文件完整还原。".to_owned())
    }
    for (n, b) in &files {
        if !seen.insert(n.clone()) {
            return Err("压缩包存在重复路径，请重新导出".into());
        };
        if backup {
            if n.starts_with("pages/") && extension(n) == "json" {
                let p: Value = serde_json::from_slice(b).map_err(err)?;
                crate::validate_page(&p)?;
                if !crate::valid_id(text(&p, "id")) || original_ids.contains_key(text(&p, "id")) {
                    return Err("备份中页面标识无效或重复".into());
                }
                let id = uid();
                original_ids.insert(text(&p, "id").to_owned(), id.clone());
                page_ids.insert(n.clone(), id);
                continue;
            }
            if !n.starts_with("assets/") {
                continue;
            }
            let asset = asset_name(n, b);
            asset_ids.insert(n.clone(), asset.clone());
            assets.insert(asset, b.clone());
            continue;
        }
        match extension(n).as_str() {
            "md" | "markdown" | "html" | "htm" | "csv" => {
                page_ids.insert(n.clone(), uid());
            }
            "json" => {
                if backup {
                    continue;
                };
                warnings.push(format!("跳过无法识别的数据文件：{n}"));
            }
            _ => {
                let asset = asset_name(n, b);
                asset_ids.insert(n.clone(), asset.clone());
                assets.insert(asset, b.clone());
            }
        }
    }
    let mut pages = vec![];
    for (n, b) in &files {
        let Some(id) = page_ids.get(n) else { continue };
        let mut p = json!({"id":id,"title":clean_title(n),"icon":"📄","tags":[],"blocks":[],"favorite":false,"deleted":false,"wide":false,"locked":false,"createdAt":now(),"updatedAt":now(),"revision":1});
        if backup {
            p = serde_json::from_slice(b).map_err(err)?;
            let old_parent = text(&p, "parentId").to_owned();
            p["id"] = json!(id);
            p["parentId"] = json!(original_ids.get(&old_parent).cloned().unwrap_or_default());
            p["revision"] = json!(1);
            p["createdAt"] = json!(now());
            p["updatedAt"] = json!(now());
            p["deleted"] = json!(false);
            if let Some(columns) = p
                .get_mut("database")
                .and_then(|db| db.get_mut("columns"))
                .and_then(Value::as_array_mut)
            {
                for column in columns {
                    let old_target = text(column, "targetDatabaseId").to_owned();
                    if let Some(new_target) = original_ids.get(&old_target) {
                        column["targetDatabaseId"] = json!(new_target);
                    }
                }
            }
            if let Some(blocks) = p["blocks"].as_array_mut() {
                for block in blocks {
                    let mut body = text(block, "text").to_owned();
                    for (old, new) in &original_ids {
                        body = body.replace(&format!("page:{old}"), &format!("page:{new}"))
                    }
                    for (old, new) in &asset_ids {
                        if let Some(old) = old.strip_prefix("assets/") {
                            body = body.replace(&format!("asset:{old}"), &format!("asset:{new}"));
                        }
                    }
                    block["text"] = json!(body)
                }
            }
        } else if extension(n) == "csv" {
            let source = std::str::from_utf8(b)
                .map_err(|_| format!("文件 {n} 不是有效的中文字符编码"))?
                .trim_start_matches('\u{feff}');
            let mut reader = csv::Reader::from_reader(source.as_bytes());
            let headers = reader
                .headers()
                .map_err(|e| format!("表格 {n} 无法读取：{e}"))?
                .clone();
            let columns:Vec<_>=headers.iter().enumerate().map(|(i,h)|json!({"id":format!("col{i}"),"name":h,"type":if h.contains("状态"){"select"}else if h.contains("日期"){"date"}else{"text"}})).collect();
            let mut rows = vec![];
            for record in reader.records() {
                let record = record.map_err(|e| format!("表格 {n} 无法读取：{e}"))?;
                let values = record
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (format!("col{i}"), json!(v)))
                    .collect::<serde_json::Map<_, _>>();
                rows.push(json!({"id":uid(),"values":values}));
            }
            p["database"] = json!({"columns":columns,"rows":rows});
            p["icon"] = json!("🗂️");
        } else {
            let mut body = std::str::from_utf8(b)
                .map_err(|_| format!("文件 {n} 不是有效的字符编码"))?
                .trim_start_matches('\u{feff}')
                .to_owned();
            if ["html", "htm"].contains(&extension(n).as_str()) {
                body = html_markdown(&body)
            }
            body = rewrite(&body, n, &page_ids, &asset_ids);
            let mut blocks = crate::markdown::parse(&body);
            if blocks
                .first()
                .map(|b| text(b, "type") == "h1")
                .unwrap_or(false)
            {
                p["title"] = blocks.remove(0)["text"].clone()
            };
            p["blocks"] = json!(blocks)
        }
        if !backup {
            let mut parent = Path::new(n).parent();
            'parents: while let Some(path) = parent {
                if path.as_os_str().is_empty() {
                    break;
                };
                for ext in ["md", "markdown", "html", "htm"] {
                    if let Some(id) = page_ids.get(&format!("{}.{}", path.to_string_lossy(), ext)) {
                        p["parentId"] = json!(id);
                        break 'parents;
                    }
                }
                parent = path.parent();
            }
        }
        pages.push(p)
    }
    crate::validate_hierarchy(&pages)?;
    if pages.is_empty() {
        return Err("没有找到可导入的笔记，请选择 Notion 导出文件或轻笺备份".into());
    };
    let stage = dir.join(format!(".import-{}", uid()));
    fs::create_dir(&stage).map_err(err)?;
    let mut published: Vec<PathBuf> = vec![];
    let outcome: Result<()> = (|| {
        for p in &pages {
            write_json(&stage.join(format!("{}.json", text(p, "id"))), p)?;
        }
        for (name, data) in &assets {
            atomic(&stage.join(name), data)?;
        }
        for name in assets.keys() {
            let target = dir.join("assets").join(name);
            if target.exists() {
                continue;
            };
            fs::rename(stage.join(name), &target).map_err(err)?;
            published.push(target);
        }
        for p in &pages {
            let name = format!("{}.json", text(p, "id"));
            let target = dir.join("pages").join(&name);
            fs::rename(stage.join(name), &target).map_err(err)?;
            published.push(target);
        }
        Ok(())
    })();
    if outcome.is_err() {
        for p in &published {
            let _ = fs::remove_file(p);
        }
    }
    let _ = fs::remove_dir_all(stage);
    outcome?;
    Ok(json!({"pages":pages.len(),"assets":assets.len(),"warnings":warnings}))
}
fn html_markdown(input: &str) -> String {
    use ego_tree::NodeRef;
    use scraper::{Html, Node};
    fn walk(node: NodeRef<Node>, out: &mut String) {
        match node.value() {
            Node::Text(t) => out.push_str(t),
            Node::Element(e) => {
                let tag = e.name();
                if [
                    "script", "style", "head", "iframe", "object", "noscript", "svg",
                ]
                .contains(&tag)
                {
                    return;
                };
                match tag {
                    "h1" => out.push_str("\n\n# "),
                    "h2" => out.push_str("\n\n## "),
                    "h3" => out.push_str("\n\n### "),
                    "p" | "div" | "section" | "figure" | "table" | "tr" => out.push_str("\n\n"),
                    "br" => out.push('\n'),
                    "li" => out.push_str("\n- "),
                    "blockquote" => out.push_str("\n\n> "),
                    "pre" => out.push_str("\n\n```\n"),
                    "strong" | "b" => out.push_str("**"),
                    "em" | "i" => out.push('*'),
                    "a" => {
                        if e.attr("href").is_some() {
                            out.push('[')
                        }
                    }
                    "img" => {
                        if let Some(src) = e.attr("src") {
                            out.push_str(&format!("![{}]({src})", e.attr("alt").unwrap_or("")))
                        }
                    }
                    _ => (),
                };
                for c in node.children() {
                    walk(c, out)
                }
                match tag {
                    "h1" | "h2" | "h3" | "p" | "div" | "section" | "figure" | "table" | "tr"
                    | "blockquote" => out.push_str("\n\n"),
                    "pre" => out.push_str("\n```\n\n"),
                    "strong" | "b" => out.push_str("**"),
                    "em" | "i" => out.push('*'),
                    "a" => {
                        if let Some(href) = e.attr("href") {
                            out.push_str(&format!("]({href})"))
                        }
                    }
                    "td" | "th" => out.push_str(" | "),
                    _ => (),
                };
            }
            _ => {
                for c in node.children() {
                    walk(c, out)
                }
            }
        }
    }
    let html = Html::parse_document(input);
    let mut output = String::new();
    walk(html.tree.root(), &mut output);
    output
}
pub fn export(dir: &Path) -> Result<Value> {
    let pages = Store::pages_at(dir)?;
    let mut z = ZipWriter::new(Cursor::new(vec![]));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    z.start_file("manifest.json", options).map_err(err)?;
    z.write_all(br#"{"format":"qingjian","version":1}"#)
        .map_err(err)?;
    for p in pages.iter().filter(|p| p["deleted"] != true) {
        let id = text(p, "id");
        z.start_file(format!("pages/{id}.json"), options)
            .map_err(err)?;
        z.write_all(&serde_json::to_vec_pretty(p).map_err(err)?)
            .map_err(err)?;
        z.start_file(format!("markdown/{id}.md"), options)
            .map_err(err)?;
        z.write_all(
            format!(
                "# {}\n\n{}",
                text(p, "title"),
                crate::markdown::export(&p["blocks"])
            )
            .as_bytes(),
        )
        .map_err(err)?;
        if let Some(columns) = p["database"]["columns"].as_array() {
            z.start_file(format!("tables/{id}.csv"), options)
                .map_err(err)?;
            let mut csv = csv::Writer::from_writer(Vec::new());
            csv.write_record(columns.iter().map(|c| text(c, "name")))
                .map_err(err)?;
            if let Some(rows) = p["database"]["rows"].as_array() {
                for row in rows {
                    csv.write_record(columns.iter().map(|c| {
                        row["values"]
                            .get(text(c, "id"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                    }))
                    .map_err(err)?;
                }
            }
            csv.flush().map_err(err)?;
            z.write_all(&csv.into_inner().map_err(err)?).map_err(err)?;
        }
    }
    for e in fs::read_dir(dir.join("assets")).map_err(err)? {
        let e = e.map_err(err)?;
        if e.file_type().map_err(err)?.is_symlink() {
            return Err("附件包含不安全链接".into());
        };
        if e.file_type().map_err(err)?.is_file() {
            z.start_file(
                format!("assets/{}", e.file_name().to_string_lossy()),
                options,
            )
            .map_err(err)?;
            z.write_all(&fs::read(e.path()).map_err(err)?)
                .map_err(err)?;
        }
    }
    let bytes = z.finish().map_err(err)?.into_inner();
    Ok(
        json!({"name":"轻笺笔记备份.zip","data":base64::engine::general_purpose::STANDARD.encode(bytes)}),
    )
}
