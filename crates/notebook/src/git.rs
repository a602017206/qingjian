use crate::{err, text, valid_id, Result, Store};
use git2::{
    build::{CheckoutBuilder, RepoBuilder},
    Cred, CredentialType, Direction, FetchOptions, IndexAddOption, Oid, PushOptions,
    RemoteCallbacks, Repository, Signature, StatusOptions,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub fn configure_network(_root: &Path) -> Result<()> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    {
        let path = _root.join("https-roots.pem");
        crate::atomic(&path, include_bytes!("../../../resources/ca-bundle.pem"))?;
        unsafe {
            git2::opts::set_ssl_cert_file(&path).map_err(err)?;
        }
    }
    unsafe {
        git2::opts::set_server_connect_timeout_in_milliseconds(15000).map_err(err)?;
        git2::opts::set_server_timeout_in_milliseconds(60000).map_err(err)?;
    }
    Ok(())
}
pub(crate) fn validate_remote(remote: &str) -> Result<()> {
    if remote.is_empty() {
        return Ok(());
    };
    if Path::new(remote).is_absolute() {
        return Ok(());
    };
    let u = url::Url::parse(remote).map_err(|_| "请输入 HTTPS 仓库地址")?;
    if u.scheme() != "https" || u.host_str().is_none() {
        return Err("所有平台统一支持 HTTPS 仓库地址".into());
    };
    if !u.username().is_empty()
        || u.password().is_some()
        || u.query().is_some()
        || u.fragment().is_some()
    {
        return Err("仓库地址不能包含令牌、密码或查询参数，请单独设置认证".into());
    };
    Ok(())
}
fn callbacks<'a>(username: &'a str, token: &'a str) -> RemoteCallbacks<'a> {
    let mut c = RemoteCallbacks::new();
    c.credentials(move |_, _, allowed| {
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) && !token.is_empty() {
            Cred::userpass_plaintext(
                if username.is_empty() {
                    "oauth2"
                } else {
                    username
                },
                token,
            )
        } else {
            Err(git2::Error::from_str(
                "私有仓库需要访问令牌，请在工作区设置中填写本次会话认证",
            ))
        }
    });
    let started = Instant::now();
    c.transfer_progress(move |_| started.elapsed() < Duration::from_secs(90));
    c.push_update_reference(|_, status| {
        if let Some(message) = status {
            Err(git2::Error::from_str(message))
        } else {
            Ok(())
        }
    });
    c
}
pub(crate) fn clone_repo(
    remote: &str,
    branch: &str,
    dir: &Path,
    user: &str,
    token: &str,
) -> Result<()> {
    let mut f = FetchOptions::new();
    f.remote_callbacks(callbacks(user, token));
    RepoBuilder::new()
        .branch(branch)
        .fetch_options(f)
        .clone(remote, dir)
        .map_err(|e| format!("克隆失败，请检查仓库、分支与认证：{}", e.message()))?;
    Ok(())
}
fn note_path(p: &str) -> bool {
    p == "workspace.json" || p.starts_with("pages/") || p.starts_with("assets/")
}
pub(crate) fn status(dir: &Path, w: &Value) -> Result<Value> {
    let repo = Repository::open(dir).map_err(err)?;
    let mut options = StatusOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut options)).map_err(err)?;
    let changed = statuses
        .iter()
        .filter(|e| e.path().map(note_path).unwrap_or(false))
        .count();
    let head = repo.head().ok().and_then(|h| h.target());
    let remote = repo
        .find_reference(&format!("refs/remotes/origin/{}", text(w, "branch")))
        .ok()
        .and_then(|r| r.target());
    let (ahead, behind) = if let (Some(a), Some(b)) = (head, remote) {
        repo.graph_ahead_behind(a, b).unwrap_or_default()
    } else {
        (0, 0)
    };
    Ok(
        json!({"branch":text(w,"branch"),"remote":text(w,"remote"),"changed":changed,"commit":head.map(|id|id.to_string()).unwrap_or_default(),"ahead":ahead,"behind":behind,"message":if text(w,"remote").is_empty(){"仅本地保存"}else{"已关联远程仓库"}}),
    )
}
pub(crate) fn commit(dir: &Path, message: &str) -> Result<Value> {
    let repo = Repository::open(dir).map_err(err)?;
    let mut index = repo.index().map_err(err)?;
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let tree = parent.as_ref().and_then(|c| c.tree().ok());
    let staged = repo
        .diff_tree_to_index(tree.as_ref(), Some(&index), None)
        .map_err(err)?;
    for delta in staged.deltas() {
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(|p| p.to_str())
            .unwrap_or("");
        if !note_path(path) {
            return Err("仓库有笔记以外的暂存文件，请先处理后再保存版本".into());
        }
    }
    drop(staged);
    index
        .add_all(
            ["pages", "assets", "workspace.json"].iter(),
            IndexAddOption::DEFAULT,
            None,
        )
        .map_err(err)?;
    index.write().map_err(err)?;
    let tree_id = index.write_tree().map_err(err)?;
    if parent.as_ref().map(|p| p.tree_id()) == Some(tree_id) {
        return Ok(
            json!({"commit":parent.unwrap().id().to_string(),"message":"没有需要提交的修改"}),
        );
    };
    let tree = repo.find_tree(tree_id).map_err(err)?;
    let sig = Signature::now("轻笺用户", "local@qingjian.invalid").map_err(err)?;
    let parents: Vec<_> = parent.iter().collect();
    let oid = repo
        .commit(
            Some("HEAD"),
            &sig,
            &sig,
            if message.is_empty() {
                "保存笔记"
            } else {
                message
            },
            &tree,
            &parents,
        )
        .map_err(err)?;
    Ok(json!({"commit":oid.to_string(),"message":"已保存本地版本"}))
}
pub(crate) fn sync(dir: &Path, w: &Value, user: &str, token: &str) -> Result<Value> {
    let local = commit(dir, "同步前保存笔记")?;
    if text(w, "remote").is_empty() {
        return Ok(
            json!({"commit":local["commit"],"message":"已保存本地版本；关联远程仓库后可同步"}),
        );
    };
    let repo = Repository::open(dir).map_err(err)?;
    let mut remote = repo.find_remote("origin").map_err(err)?;
    let branch = text(w, "branch");
    let refname = format!("refs/heads/{branch}");
    let exists = {
        let connection = remote
            .connect_auth(Direction::Fetch, Some(callbacks(user, token)), None)
            .map_err(|e| format!("连接远程失败，请检查网络和认证：{}", e.message()))?;
        connection
            .list()
            .map_err(err)?
            .iter()
            .any(|h| h.name() == refname)
    };
    if exists {
        let tracking = format!("refs/remotes/origin/{branch}");
        let mut options = FetchOptions::new();
        options.remote_callbacks(callbacks(user, token));
        remote
            .fetch(
                &[&format!("+{refname}:{tracking}")],
                Some(&mut options),
                None,
            )
            .map_err(|e| format!("拉取失败：{}", e.message()))?;
        let theirs = repo
            .find_reference(&tracking)
            .map_err(err)?
            .target()
            .ok_or("远程版本无法读取")?;
        let ours = repo
            .head()
            .map_err(err)?
            .target()
            .ok_or("本地版本无法读取")?;
        if theirs != ours {
            if repo.graph_descendant_of(theirs, ours).map_err(err)? {
                let target = repo.find_commit(theirs).map_err(err)?;
                validate_tree(&repo, &target.tree().map_err(err)?)?;
                repo.checkout_tree(target.as_object(), Some(CheckoutBuilder::new().safe()))
                    .map_err(|e| format!("远程更新无法应用，本地内容已保留：{}", e.message()))?;
                repo.find_reference(&refname)
                    .map_err(err)?
                    .set_target(theirs, "快进更新笔记")
                    .map_err(err)?;
                repo.set_head(&refname).map_err(err)?;
                Store::pages_at(dir)?;
            } else if !repo.graph_descendant_of(ours, theirs).map_err(err)? {
                return Err("本地与远程历史已分叉，已保留本地提交。请手动合并后重试，或新建工作区克隆远程版本；不会自动覆盖或强制推送".into());
            }
        }
    }
    let mut push = PushOptions::new();
    push.remote_callbacks(callbacks(user, token));
    remote
        .push(&[&format!("{refname}:{refname}")], Some(&mut push))
        .map_err(|e| format!("推送失败，本地版本已保留：{}", e.message()))?;
    let head = repo
        .head()
        .map_err(err)?
        .target()
        .map(|id| id.to_string())
        .unwrap_or_default();
    Ok(json!({"commit":head,"message":"已与远程仓库同步"}))
}
fn validate_tree(repo: &Repository, tree: &git2::Tree) -> Result<()> {
    let mut forbidden = false;
    tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
        if entry.filemode() == 0o120000 || entry.filemode() == 0o160000 {
            forbidden = true;
        }
        git2::TreeWalkResult::Ok
    })
    .map_err(err)?;
    if forbidden {
        return Err("远程仓库包含符号链接或子模块，拒绝更新".into());
    }
    let mut all_pages = vec![];
    if let Ok(entry) = tree.get_path(Path::new("pages")) {
        let object = entry.to_object(repo).map_err(err)?;
        let pages = object.as_tree().ok_or("远程页面目录无效")?;
        for p in pages.iter() {
            if p.filemode() == 0o120000 {
                return Err("远程笔记包含符号链接，拒绝更新".into());
            };
            if p.name().unwrap_or("").ends_with(".json") {
                let blob = repo.find_blob(p.id()).map_err(err)?;
                if blob.size() > 16 << 20 {
                    return Err("远程页面超过大小限制，已保留当前版本".into());
                }
                let page: Value = serde_json::from_slice(blob.content())
                    .map_err(|_| "远程笔记文件损坏，拒绝更新")?;
                crate::validate_page(&page)?;
                if !valid_id(text(&page, "id"))
                    || p.name().unwrap_or("") != format!("{}.json", text(&page, "id"))
                {
                    return Err("远程页面格式无效，拒绝更新".into());
                }
                all_pages.push(page);
            }
        }
    }
    crate::validate_hierarchy(&all_pages)
}
pub(crate) fn history(dir: &Path, id: &str) -> Result<Value> {
    if !valid_id(id) {
        return Err("页面标识无效".into());
    };
    let repo = Repository::open(dir).map_err(err)?;
    let mut walk = repo.revwalk().map_err(err)?;
    if walk.push_head().is_err() {
        return Ok(json!([]));
    };
    walk.set_sorting(git2::Sort::TIME).map_err(err)?;
    let path = format!("pages/{id}.json");
    let mut results = vec![];
    for oid in walk {
        let commit = repo.find_commit(oid.map_err(err)?).map_err(err)?;
        let tree = commit.tree().map_err(err)?;
        let current = tree.get_path(Path::new(&path)).ok().map(|e| e.id());
        let previous = commit
            .parent(0)
            .ok()
            .and_then(|c| c.tree().ok())
            .and_then(|t| t.get_path(Path::new(&path)).ok().map(|e| e.id()));
        if current.is_some() && current != previous {
            let date = chrono::DateTime::from_timestamp(commit.time().seconds(), 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            results.push(json!({"commit":commit.id().to_string(),"author":commit.author().name().unwrap_or(""),"date":date,"message":commit.summary().ok().flatten().unwrap_or("")}));
            if results.len() >= 50 {
                break;
            }
        }
    }
    Ok(json!(results))
}
pub(crate) fn read_version(dir: &Path, id: &str, oid: &str) -> Result<Value> {
    if !valid_id(id) || oid.len() != 40 || !oid.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("版本标识无效".into());
    };
    let repo = Repository::open(dir).map_err(err)?;
    let commit = repo
        .find_commit(Oid::from_str(oid).map_err(err)?)
        .map_err(err)?;
    let entry = commit
        .tree()
        .map_err(err)?
        .get_path(Path::new(&format!("pages/{id}.json")))
        .map_err(|_| "此版本没有该页面")?;
    let blob = repo.find_blob(entry.id()).map_err(err)?;
    serde_json::from_slice(blob.content()).map_err(err)
}
