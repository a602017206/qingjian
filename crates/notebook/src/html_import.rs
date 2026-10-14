use crate::{text, uid, Result};
use ego_tree::NodeRef;
use regex::Regex;
use scraper::{ElementRef, Html, Node, Selector};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
#[derive(Clone)]
pub(crate) struct Page {
    pub title: String,
    pub icon: String,
    pub source_id: String,
    pub body: String,
    pub properties: Vec<Value>,
    pub database: Option<Value>,
    pub warnings: Vec<String>,
}
fn sel(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
fn clean_text(node: NodeRef<Node>, out: &mut String) {
    match node.value() {
        Node::Text(t) => out.push_str(t),
        Node::Element(e) => {
            if [
                "script", "style", "head", "iframe", "object", "noscript", "svg",
            ]
            .contains(&e.name())
                || e.classes()
                    .any(|c| c == "property-icon" || c == "status-dot")
            {
                return;
            }
            for child in node.children() {
                clean_text(child, out);
            }
            if e.name() == "br" {
                out.push('\n');
            }
        }
        _ => {
            for child in node.children() {
                clean_text(child, out)
            }
        }
    }
}
fn plain(e: ElementRef<'_>) -> String {
    let mut s = String::new();
    clean_text(*e, &mut s);
    s.trim().to_owned()
}
pub(crate) fn source_id(value: &str) -> String {
    let id = value.replace('-', "").to_ascii_lowercase();
    if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
        id
    } else {
        String::new()
    }
}
fn color(e: ElementRef<'_>) -> String {
    let value = e
        .value()
        .classes()
        .find_map(|c| c.strip_prefix("select-value-color-"))
        .unwrap_or("gray");
    if [
        "gray", "green", "purple", "yellow", "red", "blue", "brown", "pink", "orange",
    ]
    .contains(&value)
    {
        value.into()
    } else {
        "gray".into()
    }
}
fn icon_type(header: ElementRef<'_>) -> &'static str {
    let image = header
        .select(&sel("img[src]"))
        .next()
        .and_then(|e| e.value().attr("src"))
        .unwrap_or("");
    if image.contains("burst_") {
        "status"
    } else if image.contains("calendar_") {
        "date"
    } else if image.contains("list_") {
        "multiselect"
    } else if image.contains("arrow-circle-down_") {
        "select"
    } else if image.contains("hashtag_") {
        "number"
    } else if image.contains("checkbox_") {
        "checkbox"
    } else {
        "text"
    }
}
fn class_type(row: ElementRef<'_>) -> &'static str {
    let value = row
        .value()
        .classes()
        .find_map(|c| c.strip_prefix("property-row-"))
        .unwrap_or("text");
    match value {
        "multi_select" => "multiselect",
        "status" => "status",
        "select" => "select",
        "date" => "date",
        "number" => "number",
        "checkbox" => "checkbox",
        _ => "text",
    }
}
fn cell(cell: ElementRef<'_>, kind: &str) -> Value {
    let options: Vec<_> = cell
        .select(&sel(".selected-value, .status-value"))
        .map(|e| (plain(e), color(e)))
        .filter(|(v, _)| !v.is_empty())
        .collect();
    let raw = if options.is_empty() {
        plain(cell)
    } else {
        options
            .iter()
            .map(|(v, _)| v.clone())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let times: Vec<_> = cell
        .select(&sel("time[datetime]"))
        .filter_map(|e| e.value().attr("datetime"))
        .collect();
    let value = match kind {
        "multiselect" => {
            serde_json::to_string(&options.iter().map(|(v, _)| v.clone()).collect::<Vec<_>>())
                .unwrap()
        }
        "date" if times.len() == 1 => times[0].to_owned(),
        "checkbox" => cell
            .select(&sel("input[type=checkbox]"))
            .next()
            .map(|e| e.value().attr("checked").is_some())
            .unwrap_or_else(|| cell.select(&sel(".checkbox-on")).next().is_some())
            .to_string(),
        _ => raw.clone(),
    };
    let colors: serde_json::Map<String, Value> =
        options.iter().map(|(v, c)| (v.clone(), json!(c))).collect();
    json!({"type":kind,"value":value,"raw":raw,"options":options.iter().map(|(v,_)|v).collect::<Vec<_>>(),"optionColors":colors,"datetime":times,"linkedContent":linked_content(cell)})
}
fn linked_content(element: ElementRef<'_>) -> String {
    if element.select(&sel("a[href], img[src]")).next().is_some() {
        crate::import::html_body_markdown(&element.inner_html(), false)
    } else {
        String::new()
    }
}
pub(crate) fn parse(input: &str) -> Option<Page> {
    let doc = Html::parse_document(input);
    let article = doc.select(&sel("article.page")).next()?;
    let title = article.select(&sel("h1.page-title")).next().map(plain)?;
    let icon = article
        .value()
        .attr("data-notion-page-icon")
        .or_else(|| {
            doc.select(&sel("meta[name=data-notion-page-icon]"))
                .next()
                .and_then(|e| e.value().attr("content"))
        })
        .or_else(|| {
            article
                .select(&sel("[data-emoji]"))
                .next()
                .and_then(|e| e.value().attr("data-emoji"))
        })
        .unwrap_or("")
        .to_owned();
    let mut properties = vec![];
    for row in article.select(&sel("table.properties tr")) {
        let Some(header) = row.select(&sel("th")).next() else {
            continue;
        };
        let Some(value) = row.select(&sel("td")).next() else {
            continue;
        };
        let mut property = cell(value, class_type(row));
        property["name"] = json!(plain(header));
        properties.push(property);
    }
    let tables: Vec<_> = article.select(&sel("table.collection-content")).collect();
    let mut warnings = vec![];
    let database = if tables.len() == 1 {
        let table = tables[0];
        let headers: Vec<_> = table.select(&sel("thead th")).collect();
        if headers.is_empty() {
            None
        } else {
            let mut columns:Vec<Value>=headers.iter().enumerate().map(|(i,h)|json!({"id":format!("col{i}"),"name":plain(*h),"type":if i==0{"text"}else{icon_type(*h)},"options":[],"optionColors":{},"htmlType":if i==0{"text"}else{icon_type(*h)}})).collect();
            let mut rows = vec![];
            for row in table.select(&sel("tbody > tr")) {
                let cells: Vec<_> = row
                    .children()
                    .filter_map(ElementRef::wrap)
                    .filter(|e| e.value().name() == "td")
                    .collect();
                if cells.len() != columns.len() {
                    warnings.push("网页表格列数不一致，已保留原网页内容，未转换为数据库。".into());
                    rows.clear();
                    break;
                }
                let mut values = serde_json::Map::new();
                let mut raw = serde_json::Map::new();
                for (i, value) in cells.iter().enumerate() {
                    if value.select(&sel(".status-value")).next().is_some() {
                        columns[i]["type"] = json!("status");
                        columns[i]["htmlType"] = json!("status");
                    }
                    let id = text(&columns[i], "id").to_owned();
                    let c = cell(*value, text(&columns[i], "type"));
                    values.insert(id.clone(), c["value"].clone());
                    raw.insert(id, c["raw"].clone());
                    for option in c["options"].as_array().unwrap() {
                        if !columns[i]["options"].as_array().unwrap().contains(option) {
                            columns[i]["options"]
                                .as_array_mut()
                                .unwrap()
                                .push(option.clone());
                        }
                        if let Some(v) = c["optionColors"].get(option.as_str().unwrap()) {
                            columns[i]["optionColors"][option.as_str().unwrap()] = v.clone();
                        }
                    }
                }
                let anchor = cells[0].select(&sel("a[href]")).next();
                let href = anchor.and_then(|e| e.value().attr("href")).unwrap_or("");
                let source = source_id(
                    anchor
                        .and_then(|e| e.value().attr("data-notion-page-id"))
                        .or_else(|| row.value().attr("id"))
                        .unwrap_or(""),
                );
                rows.push(json!({"id":uid(),"values":values,"sourceValues":raw,"sourceId":source,"sourceHref":href}));
            }
            if rows.is_empty() && table.select(&sel("tbody > tr")).next().is_some() {
                None
            } else {
                Some(json!({"columns":columns,"rows":rows}))
            }
        }
    } else {
        if tables.len() > 1 {
            warnings.push("网页包含多个数据库表格，已保留网页正文，未猜测合并规则。".into());
        }
        None
    };
    let mut body = String::new();
    if let Some(cover) = article
        .select(&sel(".page-cover-image[src]"))
        .next()
        .and_then(|e| e.value().attr("src"))
    {
        body.push_str(&format!("[封面]({cover})\n\n"));
    }
    if let Some(desc) = article.select(&sel(".page-description")).next() {
        let description = plain(desc);
        if !description.is_empty() {
            body.push_str(&description);
            body.push_str("\n\n");
        }
    }
    for p in &properties {
        let linked = text(p, "linkedContent");
        body.push_str(&format!(
            "{}: {}\n",
            text(p, "name"),
            if linked.is_empty() {
                text(p, "raw")
            } else {
                linked
            }
        ));
    }
    if !properties.is_empty() {
        body.push_str("\n\n");
    }
    if let Some(content) = article.select(&sel(".page-body")).next() {
        body.push_str(&crate::import::html_body_markdown(
            &content.inner_html(),
            database.is_some(),
        ));
    }
    if database.is_some() {
        let table = tables[0];
        let headers: Vec<_> = table.select(&sel("thead th")).map(plain).collect();
        let mut resources = String::new();
        for row in table.select(&sel("tbody > tr")) {
            let cells: Vec<_> = row
                .children()
                .filter_map(ElementRef::wrap)
                .filter(|e| e.value().name() == "td")
                .collect();
            for (i, cell) in cells.iter().enumerate().skip(1) {
                let content = linked_content(*cell);
                if !content.is_empty() {
                    resources.push_str(&format!(
                        "\n\n{} · {}\n\n{}",
                        plain(cells[0]),
                        headers[i],
                        content
                    ));
                }
            }
        }
        if !resources.is_empty() {
            body.push_str(&resources);
            warnings.push(
                "数据库单元格中的链接和图片已按记录及字段保留在页面正文，可继续查看。".into(),
            );
        }
    }
    Some(Page {
        title,
        icon,
        source_id: source_id(article.value().attr("id").unwrap_or("")),
        body,
        properties,
        database,
        warnings,
    })
}
fn normalized(value: &str, kind: &str) -> String {
    match kind {
        "multiselect" => {
            let values = serde_json::from_str::<Vec<String>>(value).unwrap_or_else(|_| {
                value
                    .split([',', '，'])
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned)
                    .collect()
            });
            serde_json::to_string(&values).unwrap()
        }
        "date" => {
            let s = value.trim().replace('/', "-");
            chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d")
                .map(|d| d.to_string())
                .unwrap_or(s)
        }
        "number" => decimal_key(value),
        "checkbox" => match value.trim().to_ascii_lowercase().as_str() {
            "yes" | "true" | "1" => "true".into(),
            "no" | "false" | "0" | "" => "false".into(),
            _ => value.trim().into(),
        },
        _ => value.trim().into(),
    }
}
fn decimal_key(value: &str) -> String {
    let raw = value.trim();
    let unsigned = raw.strip_prefix(['+', '-']).unwrap_or(raw);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if (whole.is_empty() && fraction.is_empty())
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return raw.to_owned();
    }
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let fraction = fraction.trim_end_matches('0');
    let sign = if raw.starts_with('-') && (whole != "0" || !fraction.is_empty()) {
        "-"
    } else {
        ""
    };
    if fraction.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{fraction}")
    }
}
pub(crate) fn property_matches(raw: &str, property: &Value) -> bool {
    let kind = text(property, "type");
    normalized(raw, kind) == normalized(text(property, "value"), kind)
}
fn matching(view: &Value, full: &Value) -> Option<Vec<usize>> {
    let vc = view["columns"].as_array()?;
    let fc = full["columns"].as_array()?;
    if vc.is_empty()
        || fc.is_empty()
        || text(&vc[0], "name") != text(&fc[0], "name")
        || vc
            .iter()
            .map(|c| text(c, "name"))
            .collect::<HashSet<_>>()
            .len()
            != vc.len()
        || fc
            .iter()
            .map(|c| text(c, "name"))
            .collect::<HashSet<_>>()
            .len()
            != fc.len()
    {
        return None;
    }
    let positions = vc
        .iter()
        .map(|c| fc.iter().position(|f| text(f, "name") == text(c, "name")))
        .collect::<Option<Vec<_>>>()?;
    let mut candidates: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, row) in full["rows"].as_array()?.iter().enumerate().rev() {
        let key = vc
            .iter()
            .zip(&positions)
            .map(|(v, &j)| {
                normalized(
                    text(&row["sourceValues"], text(&fc[j], "id")),
                    text(v, "type"),
                )
            })
            .collect();
        candidates.entry(key).or_default().push(i);
    }
    let mut order = vec![];
    for row in view["rows"].as_array()? {
        let key = vc
            .iter()
            .map(|v| normalized(text(&row["values"], text(v, "id")), text(v, "type")))
            .collect::<Vec<_>>();
        let matches = candidates.get_mut(&key)?;
        if matches.len() != 1 {
            return None;
        }
        order.push(matches.pop()?);
    }
    Some(order)
}
pub(crate) fn merge_csv(page: &mut Page, data: &[u8], name: &str) -> Result<bool> {
    let Some(view) = page.database.as_ref() else {
        return Ok(false);
    };
    let mut full = crate::import::csv_database(name, data, &mut vec![])?;
    let Some(order) = matching(view, &full) else {
        return Ok(false);
    };
    let vc = view["columns"].as_array().unwrap();
    let original_rows = full["rows"].as_array().unwrap().clone();
    let cols = full["columns"].as_array_mut().unwrap();
    for col in cols.iter_mut() {
        if let Some(source) = vc.iter().find(|v| text(v, "name") == text(col, "name")) {
            let id = col["id"].clone();
            *col = source.clone();
            col["id"] = id;
        }
    }
    cols.sort_by_key(|c| {
        vc.iter()
            .position(|v| text(v, "name") == text(c, "name"))
            .unwrap_or(vc.len())
    });
    let columns = cols.clone();
    let rows = full["rows"].as_array_mut().unwrap();
    let used: HashSet<_> = order.iter().copied().collect();
    *rows = order
        .iter()
        .map(|&i| original_rows[i].clone())
        .chain(
            original_rows
                .iter()
                .enumerate()
                .filter(|(i, _)| !used.contains(i))
                .map(|(_, r)| r.clone()),
        )
        .collect();
    for row in rows.iter_mut() {
        for col in &columns {
            if col.get("htmlType").is_some() {
                let id = text(col, "id");
                row["values"][id] = json!(normalized(
                    text(&row["sourceValues"], id),
                    text(col, "type")
                ));
            }
        }
    }
    for (row, vrow) in rows.iter_mut().zip(view["rows"].as_array().unwrap()) {
        for vcol in vc {
            let target = columns
                .iter()
                .find(|c| text(c, "name") == text(vcol, "name"))
                .unwrap();
            row["values"][text(target, "id")] = vrow["values"][text(vcol, "id")].clone();
        }
        row["sourceId"] = vrow["sourceId"].clone();
        row["sourceHref"] = vrow["sourceHref"].clone();
    }
    page.database = Some(full);
    Ok(true)
}
pub(crate) fn prepare(
    files: &[(String, Vec<u8>)],
    csv_pairs: &HashMap<String, String>,
) -> Result<(HashMap<String, Page>, HashMap<String, String>)> {
    let mut pages = HashMap::new();
    let mut aliases = HashMap::new();
    let filename_id = Regex::new(r"(?i)\s([0-9a-f]{32})$").unwrap();
    for (n, b) in files {
        if !matches!(
            Path::new(n)
                .extension()
                .and_then(|v| v.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("html" | "htm")
        ) {
            continue;
        }
        let source = std::str::from_utf8(b).map_err(|_| "网页文件编码无效")?;
        if let Some(mut page) = parse(source) {
            let csv = Path::new(n)
                .with_extension("csv")
                .to_string_lossy()
                .to_string();
            let csv = csv_pairs.get(&csv).unwrap_or(&csv);
            if let Some((_, data)) = files.iter().find(|(name, _)| name == csv) {
                let file_id = filename_id
                    .captures(
                        Path::new(n)
                            .file_stem()
                            .and_then(|v| v.to_str())
                            .unwrap_or(""),
                    )
                    .map(|c| source_id(&c[1]))
                    .unwrap_or_default();
                if !file_id.is_empty() && !page.source_id.is_empty() && file_id != page.source_id {
                    page.warnings
                        .push("网页标识与导出文件标识冲突，网页和配套表格分别保留。".into());
                } else if merge_csv(&mut page, data, csv)? {
                    aliases.insert(csv.clone(), n.clone());
                } else if page.database.is_some() {
                    page.warnings
                        .push("网页数据库与配套表格内容不一致，分别保留，未猜测归并。".into());
                }
            }
            pages.insert(n.clone(), page);
        }
    }
    Ok((pages, aliases))
}
pub(crate) fn finish(
    pages: &mut [Value],
    paths: &HashMap<String, String>,
    warnings: &mut Vec<String>,
) {
    let by_source: HashMap<String, Vec<usize>> = {
        let mut index: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, p) in pages.iter().enumerate() {
            let id = text(p, "importNotionId");
            if !id.is_empty() {
                index.entry(id.to_owned()).or_default().push(i);
            }
        }
        index
    };
    let mut links = vec![];
    for (i, parent) in pages.iter().enumerate() {
        let Some(db) = parent["database"].as_object() else {
            continue;
        };
        let rows = db["rows"].as_array().unwrap();
        for (j, row) in rows.iter().enumerate() {
            let source = text(row, "sourceId");
            let href = text(row, "sourceHref");
            let href_target = if href.is_empty() {
                None
            } else {
                paths.get(&crate::import::resolve(text(parent, "importSource"), href))
            };
            let candidates = if let Some(id) = href_target {
                pages
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        text(p, "id") == id
                            && (source.is_empty() || text(p, "importNotionId") == source)
                    })
                    .map(|(k, _)| k)
                    .collect::<Vec<_>>()
            } else {
                by_source.get(source).cloned().unwrap_or_default()
            };
            if candidates.len() == 1 {
                let child = candidates[0];
                if child != i && text(&pages[child], "parentId") == text(parent, "id") {
                    links.push((i, j, child));
                }
            } else if !source.is_empty() || !href.is_empty() {
                warnings.push("网页记录的子页面引用无法唯一匹配，已保留数据。".into());
            }
        }
    }
    let mut counts = HashMap::new();
    for &(parent, _, child) in &links {
        *counts.entry((parent, child)).or_insert(0usize) += 1;
    }
    for (parent, row, child) in links {
        if counts[&(parent, child)] != 1 {
            warnings.push("多个网页记录指向同一子页面，未猜测关联。".into());
            continue;
        }
        let properties = pages[child]["importProperties"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let id = pages[child]["id"].clone();
        pages[parent]["database"]["rows"][row]["pageId"] = id;
        for p in properties {
            let name = text(&p, "name");
            let kind = text(&p, "type");
            let Some(index) = pages[parent]["database"]["columns"]
                .as_array()
                .unwrap()
                .iter()
                .position(|c| text(c, "name") == name)
            else {
                continue;
            };
            let original_kind =
                text(&pages[parent]["database"]["columns"][index], "htmlType").to_owned();
            if !original_kind.is_empty() && original_kind != "text" && original_kind != kind {
                warnings.push(format!(
                    "网页字段 {name} 类型存在冲突，采用父页面定义并保留原属性。"
                ));
                continue;
            }
            let previous_kind =
                text(&pages[parent]["database"]["columns"][index], "type").to_owned();
            let col = &mut pages[parent]["database"]["columns"][index];
            let colid = text(col, "id").to_owned();
            col["type"] = json!(kind);
            col["htmlType"] = json!(kind);
            if matches!(kind, "select" | "status" | "multiselect") {
                if !col["options"].is_array() {
                    col["options"] = json!([]);
                }
                if !col["optionColors"].is_object() {
                    col["optionColors"] = json!({});
                }
                for value in p["options"].as_array().unwrap() {
                    if !col["options"].as_array().unwrap().contains(value) {
                        col["options"].as_array_mut().unwrap().push(value.clone());
                    }
                    let v = value.as_str().unwrap();
                    if col["optionColors"].get(v).is_none() {
                        col["optionColors"][v] = p["optionColors"][v].clone();
                    }
                }
            }
            if previous_kind != kind {
                for item in pages[parent]["database"]["rows"].as_array_mut().unwrap() {
                    let raw = text(&item["sourceValues"], &colid).to_owned();
                    item["values"][&colid] = json!(normalized(&raw, kind));
                }
            }
            let raw = text(
                &pages[parent]["database"]["rows"][row]["sourceValues"],
                &colid,
            )
            .to_owned();
            if !property_matches(&raw, &p) {
                warnings.push(format!(
                    "网页字段 {name} 与配套表格原值存在冲突，保留表格数据及子页面原文。"
                ));
                continue;
            }
            if kind == "multiselect" {
                pages[parent]["database"]["rows"][row]["values"][&colid] = p["value"].clone();
            } else if kind == "date" {
                pages[parent]["database"]["rows"][row]["values"][&colid] =
                    json!(normalized(&raw, kind));
            }
        }
    }
}
