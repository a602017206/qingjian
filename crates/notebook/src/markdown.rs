use crate::uid;
use serde_json::{json, Value};
pub fn parse(text: &str) -> Vec<Value> {
    let text = text.replace("\r\n", "\n");
    let lines: Vec<_> = text.lines().collect();
    // 无法以现有块结构无损表达的容器保留全文，交由共享界面完整渲染。
    let mut fence = None;
    let complex = lines.iter().any(|line| {
        if let Some((character, length, _, _)) = fence {
            if fence_end(line, character, length) {
                fence = None;
            }
            return false;
        }
        if let Some(opening) = fence_start(line) {
            fence = Some(opening);
            return false;
        }
        let trimmed = line.trim_start();
        line.starts_with("    ")
            || line.starts_with('\t')
            || trimmed.starts_with('>')
            || trimmed.starts_with('[') && trimmed.contains("]:")
            || line.len() != trimmed.len()
                && (trimmed.starts_with("- ")
                    || trimmed.starts_with("* ")
                    || trimmed.starts_with("+ ")
                    || trimmed.starts_with("> "))
            || trimmed.split_once(". ").is_some_and(|(number, _)| {
                number.len() >= 3 && number.bytes().all(|c| c.is_ascii_digit())
            })
    });
    if complex {
        return vec![json!({"id":uid(),"type":"paragraph","text":text,"markdownDocument":true})];
    }
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
        let mut order = None;
        if line.trim_start().starts_with('|') && i < lines.len() && table_separator(lines[i]) {
            kind = "table";
            let mut table = vec![line];
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                table.push(lines[i]);
                i += 1;
            }
            body = table.join("\n");
        } else if let Some((character, length, indent, info)) = fence_start(line) {
            kind = "code";
            language = info;
            let mut code = vec![];
            while i < lines.len() && !fence_end(lines[i], character, length) {
                let value = lines[i];
                let remove = value
                    .bytes()
                    .take(indent)
                    .take_while(|b| *b == b' ')
                    .count();
                code.push(&value[remove..]);
                i += 1;
            }
            if i < lines.len() {
                i += 1;
            }
            body = code.join("\n");
        } else if let Some((heading, content)) = heading(line) {
            kind = heading;
            body = content.into();
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
                order = n.parse::<u64>().ok();
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
        if let Some(number) = order {
            b["order"] = json!(number);
        }
        if kind == "code" {
            b["language"] = json!(language)
        }
        blocks.push(b)
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}
fn fence_start(line: &str) -> Option<(u8, usize, usize, &str)> {
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    if indent > 3 {
        return None;
    }
    let value = &line[indent..];
    let character = *value.as_bytes().first()?;
    if character != b'`' && character != b'~' {
        return None;
    }
    let length = value.bytes().take_while(|b| *b == character).count();
    let info = value[length..].trim();
    if length < 3 || character == b'`' && info.contains('`') {
        return None;
    }
    Some((character, length, indent, info))
}
fn fence_end(line: &str, character: u8, length: usize) -> bool {
    let indent = line.bytes().take_while(|b| *b == b' ').count();
    if indent > 3 {
        return false;
    }
    let value = &line[indent..];
    let count = value.bytes().take_while(|b| *b == character).count();
    count >= length && value[count..].trim().is_empty()
}
fn heading(line: &str) -> Option<(&'static str, &str)> {
    let value = line.trim_start_matches(' ');
    if line.len() - value.len() > 3 {
        return None;
    }
    let count = value.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&count) || !value[count..].starts_with([' ', '\t']) {
        return None;
    }
    Some((
        ["h1", "h2", "h3", "h4", "h5", "h6"][count - 1],
        value[count..].trim_start(),
    ))
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
            let kind = crate::text(b, "type");
            let depth = b["depth"].as_u64().unwrap_or(0).min(64) as usize;
            let quote_depth = b["quoteDepth"]
                .as_u64()
                .unwrap_or(if ["quote", "callout"].contains(&kind) {
                    1
                } else {
                    0
                })
                .min(64) as usize;
            let indent = "    ".repeat(depth);
            let quote = "> ".repeat(quote_depth);
            let mut marker = String::new();
            let value = match kind {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    format!("{} {body}", "#".repeat(kind[1..].parse::<usize>().unwrap()))
                }
                "bullet" => {
                    marker = "- ".into();
                    body.to_owned()
                }
                "number" => {
                    marker = format!("{}. ", b["order"].as_u64().unwrap_or(1));
                    body.to_owned()
                }
                "todo" => {
                    let prefix = if crate::text(b, "listType") == "number" {
                        format!("{}. ", b["order"].as_u64().unwrap_or(1))
                    } else {
                        "- ".into()
                    };
                    marker = format!(
                        "{prefix}[{}] ",
                        if b["checked"] == true { "x" } else { " " }
                    );
                    body.to_owned()
                }
                "code" => {
                    let info = crate::text(b, "language").replace(['\r', '\n'], " ");
                    let character = if info.contains('`') { '~' } else { '`' };
                    let mut longest = 0;
                    let mut run = 0;
                    for c in body.chars() {
                        if c == character {
                            run += 1;
                            longest = longest.max(run);
                        } else {
                            run = 0;
                        }
                    }
                    let fence = character.to_string().repeat(3.max(longest + 1));
                    format!("{fence}{info}\n{body}\n{fence}")
                }
                "divider" => "---".into(),
                _ => body.to_owned(),
            };
            value
                .split('\n')
                .enumerate()
                .map(|(index, line)| {
                    format!(
                        "{quote}{indent}{}{line}",
                        if index == 0 {
                            marker.clone()
                        } else {
                            " ".repeat(marker.len())
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
