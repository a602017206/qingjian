use crate::{text, valid_id, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(crate) fn validate(value: &Value) -> Result<()> {
    if value["schemaVersion"]
        .as_u64()
        .is_some_and(|version| version > 2)
    {
        return Ok(());
    }
    if value["schemaVersion"].as_u64() == Some(2) {
        return if text(value, "mode") == "freeform" {
            validate_freeform(value)
        } else {
            Ok(())
        };
    }
    if value["schemaVersion"].as_u64() != Some(1) || text(value, "mode") != "mindmap" {
        return Err("不支持的思维导图格式，原数据未修改".into());
    }
    if value["edges"].as_array().is_none_or(|v| !v.is_empty()) {
        return Err("思维导图连线格式无效".into());
    }
    let nodes = value["nodes"].as_array().ok_or("思维导图节点格式无效")?;
    if nodes.is_empty() || nodes.len() > 2000 {
        return Err("思维导图节点数量无效".into());
    }
    let mut parents = HashMap::new();
    let mut orders = HashSet::new();
    for node in nodes {
        let id = text(node, "id");
        let parent = text(node, "parentId");
        let page = text(node, "pageId");
        let order = node["order"].as_u64().ok_or("思维导图节点顺序无效")?;
        if !node.is_object()
            || !node["parentId"].is_string()
            || node.get("pageId").is_some_and(|v| !v.is_string())
            || node.get("unresolvedPageId").is_some_and(|v| {
                !v.is_string()
                    || (!text(node, "unresolvedPageId").is_empty()
                        && !valid_id(text(node, "unresolvedPageId")))
            })
            || !valid_id(id)
            || (!parent.is_empty() && !valid_id(parent))
            || (!page.is_empty() && !valid_id(page))
            || !node["text"].as_str().is_some_and(|s| s.len() <= 8192)
            || node.get("collapsed").is_some_and(|v| !v.is_boolean())
            || parents.insert(id, parent).is_some()
            || !orders.insert((parent, order))
        {
            return Err("思维导图节点格式无效".into());
        }
    }
    if parents.values().filter(|p| p.is_empty()).count() != 1 {
        return Err("思维导图必须有且仅有一个根节点".into());
    }
    for &id in parents.keys() {
        let mut seen = HashSet::new();
        let mut current = id;
        while !current.is_empty() {
            if !seen.insert(current) {
                return Err("思维导图不能包含循环".into());
            }
            current = parents
                .get(current)
                .copied()
                .ok_or("思维导图包含不存在的父节点")?;
        }
    }
    Ok(())
}

fn validate_freeform(value: &Value) -> Result<()> {
    let nodes = value["nodes"].as_array().ok_or("节点图节点格式无效")?;
    let edges = value["edges"].as_array().ok_or("节点图连线格式无效")?;
    if nodes.len() > 2000 || edges.len() > 4000 {
        return Err("节点图数量超出限制".into());
    }
    let mut ids = HashSet::new();
    for node in nodes {
        let id = text(node, "id");
        let page = text(node, "pageId");
        let unresolved = text(node, "unresolvedPageId");
        let coord = |key: &str| {
            node[key]
                .as_f64()
                .is_some_and(|n| n.is_finite() && n.abs() <= 1_000_000.0)
        };
        if !node.is_object()
            || !valid_id(id)
            || !ids.insert(id)
            || !node["text"].as_str().is_some_and(|s| s.len() <= 8192)
            || !coord("x")
            || !coord("y")
            || node.get("pageId").is_some_and(|v| !v.is_string())
            || (!page.is_empty() && !valid_id(page))
            || node.get("unresolvedPageId").is_some_and(|v| !v.is_string())
            || (!unresolved.is_empty() && !valid_id(unresolved))
        {
            return Err("节点图节点格式无效".into());
        }
    }
    let mut edge_ids = HashSet::new();
    for edge in edges {
        let id = text(edge, "id");
        if !edge.is_object()
            || !valid_id(id)
            || !edge_ids.insert(id)
            || !ids.contains(text(edge, "sourceId"))
            || !ids.contains(text(edge, "targetId"))
            || !edge["directed"].is_boolean()
            || !edge["label"].as_str().is_some_and(|s| s.len() <= 2048)
        {
            return Err("节点图连线格式无效".into());
        }
    }
    Ok(())
}

pub(crate) fn markdown(diagram: &Value) -> String {
    if diagram["schemaVersion"].as_u64() == Some(2) && text(diagram, "mode") == "freeform" {
        return freeform_markdown(diagram);
    }
    if diagram["schemaVersion"].as_u64() != Some(1) {
        return "[思维导图格式不受支持，请使用结构化备份保留原始数据]".into();
    }
    let Some(nodes) = diagram["nodes"].as_array() else {
        return String::new();
    };
    let mut children: HashMap<&str, Vec<&Value>> = HashMap::new();
    for node in nodes {
        children
            .entry(text(node, "parentId"))
            .or_default()
            .push(node);
    }
    for group in children.values_mut() {
        group.sort_by_key(|n| n["order"].as_u64().unwrap_or(0));
    }
    let mut stack: Vec<(&Value, usize)> = children
        .get("")
        .into_iter()
        .flat_map(|v| v.iter().rev())
        .map(|n| (*n, 0))
        .collect();
    let mut out = Vec::new();
    while let Some((node, depth)) = stack.pop() {
        let label = text(node, "text")
            .replace('\\', "\\\\")
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace(['\r', '\n'], " ");
        let page = text(node, "pageId");
        let value = if page.is_empty() {
            label
        } else {
            format!("[{label}](page:{page})")
        };
        out.push(format!("{}- {}", "  ".repeat(depth), value));
        if let Some(group) = children.get(text(node, "id")) {
            for child in group.iter().rev() {
                stack.push((child, depth + 1));
            }
        }
    }
    out.join("\n")
}

fn freeform_markdown(diagram: &Value) -> String {
    let nodes = diagram["nodes"].as_array().cloned().unwrap_or_default();
    let edges = diagram["edges"].as_array().cloned().unwrap_or_default();
    let clean = |s: &str| {
        s.replace('\\', "\\\\")
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace(['\r', '\n'], " ")
    };
    let names: HashMap<&str, String> = nodes
        .iter()
        .map(|n| (text(n, "id"), clean(text(n, "text"))))
        .collect();
    let mut lines = vec!["## 节点".to_owned()];
    for n in &nodes {
        let label = clean(text(n, "text"));
        let page = text(n, "pageId");
        let name = if page.is_empty() {
            label
        } else {
            format!("[{label}](page:{page})")
        };
        lines.push(format!("- {name} ({})", text(n, "id")));
    }
    lines.push("\n## 连线".into());
    for edge in &edges {
        let a = names
            .get(text(edge, "sourceId"))
            .map(String::as_str)
            .unwrap_or("");
        let b = names
            .get(text(edge, "targetId"))
            .map(String::as_str)
            .unwrap_or("");
        let arrow = if edge["directed"] == true {
            "→"
        } else {
            "—"
        };
        lines.push(format!(
            "- {} ({}) {} {} ({})：{}",
            a,
            text(edge, "sourceId"),
            arrow,
            b,
            text(edge, "targetId"),
            clean(text(edge, "label"))
        ));
    }
    lines.join("\n")
}

pub(crate) fn unsupported(value: &Value) -> bool {
    let Some(version) = value["schemaVersion"].as_u64() else {
        return false;
    };
    version > 2 || version == 2 && text(value, "mode") != "freeform"
}
