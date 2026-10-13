use crate::uid;
use serde_json::{json, Value};
pub fn parse(text: &str) -> Vec<Value> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<_> = text.lines().collect();
    let mut blocks = vec![];
    let mut paragraph = vec![];
    let flush = |paragraph: &mut Vec<&str>, blocks: &mut Vec<Value>| {
        if !paragraph.is_empty() {
            blocks.push(json!({"id":uid(),"type":"paragraph","text":paragraph.join("\n")}));
            paragraph.clear();
        }
    };
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        i += 1;
        if line.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        let mut kind = "paragraph";
        let mut body = line.to_owned();
        let mut checked = false;
        let mut language = "";
        if line.trim_start().starts_with('|') && i < lines.len() && table_separator(lines[i]) {
            kind = "table";
            let mut table = vec![line];
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                table.push(lines[i]);
                i += 1;
            }
            body = table.join("\n");
        } else if let Some(lang) = line.strip_prefix("```") {
            kind = "code";
            language = lang;
            let mut code = vec![];
            while i < lines.len() && !lines[i].starts_with("```") {
                code.push(lines[i]);
                i += 1
            }
            i += 1;
            body = code.join("\n");
        } else if let Some(t) = line.strip_prefix("### ") {
            kind = "h3";
            body = t.into()
        } else if let Some(t) = line.strip_prefix("## ") {
            kind = "h2";
            body = t.into()
        } else if let Some(t) = line.strip_prefix("# ") {
            kind = "h1";
            body = t.into()
        } else if line.starts_with("- [x] ")
            || line.starts_with("- [X] ")
            || line.starts_with("- [ ] ")
        {
            kind = "todo";
            checked = !line.starts_with("- [ ]");
            body = line[6..].into()
        } else if let Some(t) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            kind = "bullet";
            body = t.into()
        } else if let Some(t) = line.strip_prefix("> ") {
            kind = "quote";
            body = t.into()
        } else if line.trim() == "---" {
            kind = "divider";
            body = String::new()
        } else if let Some((n, t)) = line.split_once(". ") {
            if !n.is_empty() && n.bytes().all(|c| c.is_ascii_digit()) {
                kind = "number";
                body = t.into()
            }
        }
        if kind == "paragraph" {
            paragraph.push(line);
            continue;
        };
        flush(&mut paragraph, &mut blocks);
        let mut b = json!({"id":uid(),"type":kind,"text":body});
        if kind == "todo" {
            b["checked"] = json!(checked)
        }
        if kind == "code" {
            b["language"] = json!(language)
        }
        blocks.push(b)
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}
fn table_separator(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('|')
        && line.trim_matches('|').split('|').all(|cell| {
            let value = cell.trim().trim_matches(':');
            value.len() >= 3 && value.bytes().all(|b| b == b'-')
        })
}
pub fn export(blocks: &Value) -> String {
    blocks
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|b| {
            let body = crate::text(b, "text");
            match crate::text(b, "type") {
                "h1" => format!("# {body}"),
                "h2" => format!("## {body}"),
                "h3" => format!("### {body}"),
                "bullet" => format!("- {body}"),
                "number" => format!("1. {body}"),
                "todo" => format!(
                    "- [{}] {body}",
                    if b["checked"] == true { "x" } else { " " }
                ),
                "quote" | "callout" => format!("> {body}"),
                "code" => format!("```{}\n{body}\n```", crate::text(b, "language")),
                "divider" => "---".into(),
                _ => body.into(),
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
