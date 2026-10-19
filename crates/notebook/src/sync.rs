use crate::{err, git, now, read_json, text, uid, write_json, Result};
use git2::{build::CheckoutBuilder, Index, IndexEntry, IndexTime, Oid, Repository, Signature};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
type Files = BTreeMap<String, (Oid, u32)>;
fn files(repo: &Repository, oid: Oid) -> Result<Files> {
    let tree = repo.find_commit(oid).map_err(err)?.tree().map_err(err)?;
    git::validate_tree(repo, &tree)?;
    let mut result = Files::new();
    let mut bad = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |prefix, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            if let Ok(name) = entry.name() {
                result.insert(
                    format!("{prefix}{name}"),
                    (entry.id(), entry.filemode() as u32),
                );
            } else {
                bad = true;
            }
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(err)?;
    if bad {
        return Err("仓库文件名编码无效，未应用更新".into());
    }
    Ok(result)
}
fn tree(repo: &Repository, files: &Files) -> Result<Oid> {
    let mut index = Index::new().map_err(err)?;
    for (path, (id, mode)) in files {
        index
            .add(&IndexEntry {
                ctime: IndexTime::new(0, 0),
                mtime: IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: *mode,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: *id,
                flags: 0,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            })
            .map_err(err)?;
    }
    let oid = index.write_tree_to(repo).map_err(err)?;
    git::validate_tree(repo, &repo.find_tree(oid).map_err(err)?)?;
    Ok(oid)
}
fn summary(
    repo: &Repository,
    file: Option<&(Oid, u32)>,
    other: Option<&(Oid, u32)>,
    path: &str,
) -> Result<Value> {
    let Some((id, _)) = file else {
        return Ok(json!({"deleted":true}));
    };
    let blob = repo.find_blob(*id).map_err(err)?;
    if path.starts_with("pages/") && path.ends_with(".json") {
        let page: Value = serde_json::from_slice(blob.content()).map_err(err)?;
        let other: Value = other
            .map(|(id, _)| -> Result<Value> {
                serde_json::from_slice(repo.find_blob(*id).map_err(err)?.content()).map_err(err)
            })
            .transpose()?
            .unwrap_or(Value::Null);
        let mut out = json!({"title":page["title"],"updatedAt":page["updatedAt"],"deleted":page["deleted"],"parentId":page["parentId"],"blocks":[],"database":null});
        // 冲突面板展示可核对的摘要，完整双方版本始终保留在恢复点中。
        let other_blocks: BTreeMap<_, _> = other["blocks"]
            .as_array()
            .map(|items| items.iter().map(|b| (text(b, "id"), b)).collect())
            .unwrap_or_default();
        out["blocks"]=json!(page["blocks"].as_array().map(|a|a.iter().filter(|block| other_blocks.get(text(block,"id")).copied()!=Some(*block)).take(12).map(|b|json!({"type":b["type"],"text":text(b,"text").chars().take(500).collect::<String>()})).collect::<Vec<_>>()).unwrap_or_default());
        if let Some(db) = page.get("database").filter(|v| !v.is_null()) {
            let columns = db["columns"].as_array().cloned().unwrap_or_default();
            let rows = db["rows"].as_array().cloned().unwrap_or_default();
            let other_rows: BTreeMap<_, _> = other["database"]["rows"]
                .as_array()
                .map(|items| items.iter().map(|row| (text(row, "id"), row)).collect())
                .unwrap_or_default();
            let mut cells = vec![];
            for row in &rows {
                let old = other_rows.get(text(row, "id"));
                let title = columns
                    .first()
                    .map(|c| text(&row["values"], text(c, "id")))
                    .unwrap_or("");
                for column in &columns {
                    let key = text(column, "id");
                    let value = &row["values"][key];
                    if old.map(|r| &r["values"][key]) == Some(value) {
                        continue;
                    }
                    cells.push(json!({"label":format!("{} · {}",if title.is_empty(){"未命名记录"}else{title},text(column,"name")),"value":value.as_str().unwrap_or("").chars().take(500).collect::<String>()}));
                    if cells.len() >= 12 {
                        break;
                    }
                }
                if cells.len() >= 12 {
                    break;
                }
            }
            out["database"] = json!({"columns":columns.len(),"rows":rows.len(),"cells":cells});
        }
        Ok(out)
    } else {
        Ok(json!({"bytes":blob.size(),"object":id.to_string()}))
    }
}
pub(crate) fn backups(dir: &Path) -> Result<Value> {
    let p = Repository::open(dir)
        .map_err(err)?
        .path()
        .join("qingjian-backups.json");
    if p.exists() {
        read_json(&p)
    } else {
        Ok(json!([]))
    }
}
fn backup(repo: &Repository, dir: &Path, ours: Oid, theirs: Oid) -> Result<()> {
    let mut list = backups(dir)?;
    let items = list.as_array_mut().ok_or("恢复点列表无效")?;
    if items
        .iter()
        .any(|v| text(v, "local") == ours.to_string() && text(v, "remote") == theirs.to_string())
    {
        return Ok(());
    }
    let id = uid();
    repo.reference(
        &format!("refs/qingjian/backups/{id}/local"),
        ours,
        false,
        "保留同步前本机版本",
    )
    .map_err(err)?;
    repo.reference(
        &format!("refs/qingjian/backups/{id}/remote"),
        theirs,
        false,
        "保留同步前远程版本",
    )
    .map_err(err)?;
    items.insert(
        0,
        json!({"id":id,"createdAt":now(),"local":ours.to_string(),"remote":theirs.to_string()}),
    );
    write_json(&repo.path().join("qingjian-backups.json"), &list)
}
fn apply(repo: &Repository, w: &Value, oid: Oid) -> Result<()> {
    let commit = repo.find_commit(oid).map_err(err)?;
    git::validate_tree(repo, &commit.tree().map_err(err)?)?;
    repo.checkout_tree(
        commit.as_object(),
        Some(CheckoutBuilder::new().safe().remove_untracked(false)),
    )
    .map_err(|e| format!("更新无法应用，本机版本已保留：{}", e.message()))?;
    repo.find_reference(&format!("refs/heads/{}", text(w, "branch")))
        .map_err(err)?
        .set_target(oid, "应用已确认的同步版本")
        .map_err(err)?;
    Ok(())
}
pub(crate) fn run(
    dir: &Path,
    w: &Value,
    user: &str,
    token: &str,
    resolution: Option<&Value>,
) -> Result<Value> {
    let local = git::commit(dir, "同步前保存笔记")?;
    if text(w, "remote").is_empty() {
        return Ok(
            json!({"status":"local","commit":local["commit"],"message":"已保存本机版本；关联远程仓库后可同步"}),
        );
    }
    let repo = Repository::open(dir).map_err(err)?;
    let ours = repo
        .head()
        .map_err(err)?
        .target()
        .ok_or("本机版本无法读取")?;
    let theirs = git::fetch_head(&repo, w, user, token)?;
    if let Some(q) = resolution {
        let p = &q["preview"];
        if text(p, "localCommit") != ours.to_string()
            || text(p, "remoteCommit") != theirs.map(|o| o.to_string()).unwrap_or_default()
            || p["workspaceId"] != w["id"]
            || p["remote"] != w["remote"]
            || p["branch"] != w["branch"]
        {
            return Err("本机、远程或仓库配置已有变化，请重新同步并核对版本后选择".into());
        }
    }
    if let Some(theirs) = theirs {
        if ours != theirs {
            let remote_files = files(&repo, theirs)?;
            if repo.graph_descendant_of(theirs, ours).map_err(err)? {
                backup(&repo, dir, ours, theirs)?;
                apply(&repo, w, theirs)?;
            } else if !repo.graph_descendant_of(ours, theirs).map_err(err)? {
                let local_files = files(&repo, ours)?;
                let base = match repo.merge_base(ours, theirs) {
                    Ok(id) => Some(id),
                    Err(e) if e.code() == git2::ErrorCode::NotFound => None,
                    Err(e) => return Err(err(e)),
                };
                let base_files = base
                    .map(|id| files(&repo, id))
                    .transpose()?
                    .unwrap_or_default();
                let paths: BTreeSet<_> = local_files
                    .keys()
                    .chain(remote_files.keys())
                    .chain(base_files.keys())
                    .cloned()
                    .collect();
                let mut merged = Files::new();
                let mut conflicts = vec![];
                for path in paths {
                    let a = local_files.get(&path);
                    let b = remote_files.get(&path);
                    let c = base_files.get(&path);
                    let chosen = if a == b || b == c {
                        a
                    } else if a == c {
                        b
                    } else if path == "workspace.json" {
                        a.or(b)
                    } else {
                        conflicts.push(json!({"path":path,"local":summary(&repo,a,b,&path)?,"remote":summary(&repo,b,a,&path)?}));
                        None
                    };
                    if let Some(v) = chosen {
                        merged.insert(path, *v);
                    }
                }
                let structure_error = if conflicts.is_empty() {
                    tree(&repo, &merged).err()
                } else {
                    None
                };
                if base.is_none() || !conflicts.is_empty() || structure_error.is_some() {
                    let preview = json!({"status":"conflict","workspaceId":w["id"],"remote":w["remote"],"branch":w["branch"],"localCommit":ours.to_string(),"remoteCommit":theirs.to_string(),"unrelated":base.is_none(),"structureError":structure_error,"localPages":local_files.keys().filter(|p|p.starts_with("pages/")).count(),"remotePages":remote_files.keys().filter(|p|p.starts_with("pages/")).count(),"conflicts":conflicts,"message":"需要选择保留版本，尚未覆盖双方笔记"});
                    let Some(q) = resolution else {
                        return Ok(preview);
                    };
                    let strategy = text(q, "strategy");
                    if base.is_none() && !["local", "remote", "merge"].contains(&strategy) {
                        return Err("请先选择以本机、远程为准，或合并两边笔记".into());
                    }
                    if strategy == "local" {
                        merged = local_files.clone();
                    } else if strategy == "remote" {
                        merged = remote_files.clone();
                    } else {
                        for c in &conflicts {
                            let path = text(c, "path");
                            let selected = match text(&q["choices"], path) {
                                "local" => local_files.get(path),
                                "remote" => remote_files.get(path),
                                _ => return Err("请为每个冲突选择保留本机或远程版本".into()),
                            };
                            if let Some(v) = selected {
                                merged.insert(path.into(), *v);
                            }
                        }
                    }
                }
                // 对方清理了页面及附件时，保留下来的笔记仍必须能读取原附件。
                let mut needed_assets = std::collections::HashSet::new();
                for (path, (id, _)) in &merged {
                    if path.starts_with("pages/") && path.ends_with(".json") {
                        let page: Value =
                            serde_json::from_slice(repo.find_blob(*id).map_err(err)?.content())
                                .map_err(err)?;
                        needed_assets.extend(crate::cleanup::asset_refs(&page));
                    }
                }
                for name in needed_assets {
                    let path = format!("assets/{name}");
                    if let std::collections::btree_map::Entry::Vacant(slot) =
                        merged.entry(path.clone())
                    {
                        if let Some(q) = resolution {
                            let choice = text(&q["choices"], &path);
                            if (choice == "local" && !local_files.contains_key(&path))
                                || (choice == "remote" && !remote_files.contains_key(&path))
                            {
                                return Err(format!(
                                    "保留的笔记仍引用附件 {name}，请保留该附件或调整笔记版本"
                                ));
                            }
                        }
                        if let Some(entry) =
                            local_files.get(&path).or_else(|| remote_files.get(&path))
                        {
                            slot.insert(*entry);
                        }
                    }
                }
                // 为变化的页面推进修订，拒绝其他窗口基于旧本机版本的覆盖。
                for (path, entry) in merged.iter_mut() {
                    if path.starts_with("pages/")
                        && path.ends_with(".json")
                        && local_files.get(path) != remote_files.get(path)
                    {
                        let mut page: Value =
                            serde_json::from_slice(repo.find_blob(entry.0).map_err(err)?.content())
                                .map_err(err)?;
                        let old_revision = local_files
                            .get(path)
                            .map(|(id, _)| -> Result<u64> {
                                let old: Value = serde_json::from_slice(
                                    repo.find_blob(*id).map_err(err)?.content(),
                                )
                                .map_err(err)?;
                                Ok(old["revision"].as_u64().unwrap_or(0))
                            })
                            .transpose()?
                            .unwrap_or(0);
                        let remote_revision = remote_files
                            .get(path)
                            .map(|(id, _)| -> Result<u64> {
                                let remote: Value = serde_json::from_slice(
                                    repo.find_blob(*id).map_err(err)?.content(),
                                )
                                .map_err(err)?;
                                Ok(remote["revision"].as_u64().unwrap_or(0))
                            })
                            .transpose()?
                            .unwrap_or(0);
                        page["revision"] = json!(old_revision
                            .max(remote_revision)
                            .max(page["revision"].as_u64().unwrap_or(0))
                            .checked_add(1)
                            .ok_or("页面修订号超出范围")?);
                        entry.0 = repo
                            .blob(&serde_json::to_vec_pretty(&page).map_err(err)?)
                            .map_err(err)?;
                    }
                }
                // 先验证完整页面层级；不同文件合并后仍可能出现父子关系冲突。
                let tree_id = tree(&repo, &merged).map_err(|e| {
                    format!("所选版本无法组成完整工作区，请调整选择；双方原始版本仍保留：{e}")
                })?;
                backup(&repo, dir, ours, theirs)?;
                let signature =
                    Signature::now("轻笺用户", "local@qingjian.invalid").map_err(err)?;
                let id = repo
                    .commit(
                        None,
                        &signature,
                        &signature,
                        "合并多设备笔记",
                        &repo.find_tree(tree_id).map_err(err)?,
                        &[
                            &repo.find_commit(ours).map_err(err)?,
                            &repo.find_commit(theirs).map_err(err)?,
                        ],
                    )
                    .map_err(err)?;
                apply(&repo, w, id)?;
            }
        }
    }
    git::push(&repo, w, user, token)?;
    Ok(
        json!({"status":"synced","commit":repo.head().map_err(err)?.target().map(|o|o.to_string()),"message":"已与远程仓库同步"}),
    )
}

pub(crate) fn export_backup(source: &Path, id: &str, side: &str, target: &Path) -> Result<()> {
    if !crate::valid_id(id) || !["local", "remote"].contains(&side) {
        return Err("恢复点标识无效".into());
    }
    let repo = Repository::open(source).map_err(err)?;
    let oid = repo
        .find_reference(&format!("refs/qingjian/backups/{id}/{side}"))
        .map_err(|_| "恢复点不存在")?
        .target()
        .ok_or("恢复点无效")?;
    let snapshot = files(&repo, oid)?;
    for (path, (id, _)) in snapshot {
        // 新工作区只导出笔记和附件，不携带源仓库配置及认证。
        if path != "workspace.json" && !path.starts_with("pages/") && !path.starts_with("assets/") {
            continue;
        }
        let dest = target.join(path);
        if let Some(parent) = dest.parent() {
            crate::ensure_dir(parent)?;
        }
        crate::atomic(&dest, repo.find_blob(id).map_err(err)?.content())?;
    }
    crate::Store::pages_at(target)?;
    Ok(())
}
