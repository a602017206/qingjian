use crate::{text, Result};
use serde_json::Value;
use std::collections::HashMap;

pub(crate) struct Position {
    pub parent: Option<usize>,
    pub depth: usize,
}
pub(crate) fn can_nest(block: &Value) -> bool {
    [
        "paragraph",
        "bullet",
        "number",
        "todo",
        "quote",
        "callout",
        "toggle",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "column",
    ]
    .contains(&text(block, "type"))
}
fn can_contain(parent: &Value, child: &Value) -> bool {
    if text(parent, "type") == "columns" {
        text(child, "type") == "column"
    } else {
        text(child, "type") != "column" && can_nest(parent)
    }
}
pub(crate) fn layout(blocks: &[Value]) -> Result<Vec<Position>> {
    let mut result: Vec<Position> = vec![];
    let mut by_id = HashMap::new();
    let mut path: Vec<usize> = vec![];
    for (index, block) in blocks.iter().enumerate() {
        if block
            .get("collapsed")
            .is_some_and(|value| !value.is_boolean())
        {
            return Err("折叠状态必须是布尔值".into());
        }
        let legacy_depth = if let Some(value) = block.get("depth") {
            value
                .as_u64()
                .filter(|v| *v <= 64)
                .ok_or("内容块缩进格式无效")? as usize
        } else {
            0
        };
        let parent = if let Some(value) = block.get("parentId") {
            let id = value.as_str().ok_or("父内容块标识必须是文本")?;
            if id.is_empty() {
                None
            } else {
                let parent = *by_id.get(id).ok_or("父内容块不存在或位于子块之后")?;
                if !path.contains(&parent) || !can_contain(&blocks[parent], block) {
                    return Err("内容块层级不连续或父块不能包含子内容".into());
                }
                Some(parent)
            }
        } else if legacy_depth > 0 && !path.is_empty() {
            let mut parent = Some(path[legacy_depth.min(path.len()) - 1]);
            while parent.is_some_and(|i| !can_nest(&blocks[i])) {
                parent = result[parent.unwrap()].parent;
            }
            parent
        } else {
            None
        };
        while path.last().copied() != parent && !path.is_empty() {
            path.pop();
        }
        let kind = text(block, "type");
        if ["column", "columns"].contains(&kind) && !text(block, "text").is_empty() {
            return Err("分栏容器不能包含隐藏正文".into());
        }
        if kind == "column" {
            if parent.is_none_or(|p| text(&blocks[p], "type") != "columns") {
                return Err("列必须属于分栏组".into());
            }
            if block.get("width").is_some_and(|width| {
                width
                    .as_f64()
                    .is_none_or(|v| !v.is_finite() || v <= 0.0 || v > 100.0)
            }) {
                return Err("列宽格式无效".into());
            }
        }
        let depth = parent.map(|i| result[i].depth + 1).unwrap_or(0);
        if depth > 64 {
            return Err("内容块嵌套不能超过六十四层".into());
        }
        result.push(Position { parent, depth });
        by_id.insert(text(block, "id"), index);
        path.push(index);
    }
    let mut counts = vec![0; blocks.len()];
    for position in &result {
        if let Some(parent) = position.parent {
            counts[parent] += 1;
        }
    }
    for (index, block) in blocks.iter().enumerate() {
        if text(block, "type") == "columns" && !(2..=6).contains(&counts[index]) {
            return Err("分栏组必须包含二至六列".into());
        }
    }
    Ok(result)
}
