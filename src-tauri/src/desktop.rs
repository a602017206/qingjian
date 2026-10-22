use crate::desktop_actions::{exit_decision, ActionQueue, ExitDecision};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem as Item, Submenu},
    Emitter, Manager,
};

#[derive(Default)]
pub struct DesktopRuntime {
    queue: Mutex<ActionQueue>,
    pub tray: AtomicBool,
    exiting: AtomicBool,
    approved: Mutex<BTreeSet<String>>,
}
fn text(locale: &str, key: &str) -> &'static str {
    let traditional = locale == "zh-TW";
    match (key, traditional) {
        ("app", false) => "轻笺",
        ("app", true) => "輕箋",
        ("show", false) => "显示轻笺",
        ("show", true) => "顯示輕箋",
        ("search", false) => "快速搜索",
        ("search", true) => "快速搜尋",
        ("new", false) => "新建笔记",
        ("new", true) => "新增筆記",
        ("quit", false) => "退出轻笺",
        ("quit", true) => "退出輕箋",
        ("about", false) => "关于轻笺",
        ("about", true) => "關於輕箋",
        ("hide", false) => "隐藏轻笺",
        ("hide", true) => "隱藏輕箋",
        ("others", false) => "隐藏其他应用",
        ("others", true) => "隱藏其他應用",
        ("file", false) => "笔记",
        ("file", true) => "筆記",
        ("edit", false) => "编辑",
        ("edit", true) => "編輯",
        ("undo", false) => "撤销",
        ("undo", true) => "復原",
        ("redo", false) => "重做",
        ("redo", true) => "重做",
        ("cut", false) => "剪切",
        ("cut", true) => "剪下",
        ("copy", false) => "复制",
        ("copy", true) => "複製",
        ("paste", false) => "粘贴",
        ("paste", true) => "貼上",
        ("all", false) => "全选",
        ("all", true) => "全選",
        ("window", false) => "窗口",
        ("window", true) => "視窗",
        ("min", false) => "最小化",
        ("min", true) => "最小化",
        ("max", false) => "放大窗口",
        ("max", true) => "放大視窗",
        ("full", false) => "切换全屏",
        ("full", true) => "切換全螢幕",
        ("close", false) => "关闭窗口",
        ("close", true) => "關閉視窗",
        _ => "轻笺",
    }
}
fn item(
    app: &tauri::AppHandle,
    id: &str,
    locale: &str,
    key: &str,
    shortcut: Option<&str>,
) -> tauri::Result<MenuItem<tauri::Wry>> {
    MenuItem::with_id(app, id, text(locale, key), true, shortcut)
}
pub fn show(app: &tauri::AppHandle, label: &str) {
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
pub fn request(app: &tauri::AppHandle, action: &str) {
    let windows: Vec<String> = if action == "quit" {
        app.webview_windows().keys().cloned().collect()
    } else {
        vec!["main".into()]
    };
    if windows.is_empty() {
        return;
    }
    let state = app.state::<DesktopRuntime>();
    if let Ok(mut queue) = state.queue.lock() {
        queue.push(action, windows.clone());
    }
    for label in windows {
        let _ = app.emit_to(label, "qingjian-desktop-actions", ());
    }
}
pub fn menu_event(app: &tauri::AppHandle, id: &str) {
    match id {
        "desktop-show" => show(app, "main"),
        "desktop-search" | "desktop-new" => {
            show(app, "main");
            request(
                app,
                if id == "desktop-search" {
                    "search"
                } else {
                    "new"
                },
            );
        }
        "desktop-quit" => request(app, "quit"),
        _ => {}
    }
}
pub fn install(app: &tauri::AppHandle, locale: &str) -> tauri::Result<()> {
    let application = Submenu::with_items(
        app,
        text(locale, "app"),
        true,
        &[
            &Item::about(app, Some(text(locale, "about")), None)?,
            &Item::separator(app)?,
            &Item::hide(app, Some(text(locale, "hide")))?,
            &Item::hide_others(app, Some(text(locale, "others")))?,
            &Item::separator(app)?,
            &item(app, "desktop-quit", locale, "quit", Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    let file = Submenu::with_items(
        app,
        text(locale, "file"),
        true,
        &[
            &item(app, "desktop-new", locale, "new", Some("CmdOrCtrl+N"))?,
            &item(app, "desktop-search", locale, "search", Some("CmdOrCtrl+K"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        text(locale, "edit"),
        true,
        &[
            &Item::undo(app, Some(text(locale, "undo")))?,
            &Item::redo(app, Some(text(locale, "redo")))?,
            &Item::separator(app)?,
            &Item::cut(app, Some(text(locale, "cut")))?,
            &Item::copy(app, Some(text(locale, "copy")))?,
            &Item::paste(app, Some(text(locale, "paste")))?,
            &Item::select_all(app, Some(text(locale, "all")))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        text(locale, "window"),
        true,
        &[
            &item(app, "desktop-show", locale, "show", None)?,
            &Item::separator(app)?,
            &Item::minimize(app, Some(text(locale, "min")))?,
            &Item::maximize(app, Some(text(locale, "max")))?,
            &Item::fullscreen(app, Some(text(locale, "full")))?,
            &Item::close_window(app, Some(text(locale, "close")))?,
        ],
    )?;
    app.set_menu(Menu::with_items(
        app,
        &[&application, &file, &edit, &window],
    )?)?;
    let tray_menu = Menu::with_items(
        app,
        &[
            &item(app, "desktop-show", locale, "show", None)?,
            &item(app, "desktop-search", locale, "search", None)?,
            &item(app, "desktop-new", locale, "new", None)?,
            &Item::separator(app)?,
            &item(app, "desktop-quit", locale, "quit", None)?,
        ],
    )?;
    if let Some(tray) = app.tray_by_id("qingjian") {
        tray.set_menu(Some(tray_menu))?;
        tray.set_tooltip(Some(text(locale, "app")))?;
        return Ok(());
    }
    if let Some(icon) = app.default_window_icon() {
        let result = tauri::tray::TrayIconBuilder::with_id("qingjian")
            .icon(icon.clone())
            .tooltip(text(locale, "app"))
            .menu(&tray_menu)
            .show_menu_on_left_click(true)
            .build(app);
        match result {
            Ok(_) => {
                app.state::<DesktopRuntime>()
                    .tray
                    .store(true, Ordering::SeqCst);
            }
            Err(error) => {
                eprintln!("系统托盘暂不可用，关闭窗口将沿用系统行为：{error}");
            }
        }
    }
    Ok(())
}
pub fn close_requested(window: &tauri::Window, api: &tauri::CloseRequestApi) {
    let state = window.state::<DesktopRuntime>();
    if !state.exiting.load(Ordering::SeqCst) {
        api.prevent_close();
        let action = if window.label() == "main" && state.tray.load(Ordering::SeqCst) {
            "hide"
        } else {
            "close"
        };
        if let Ok(mut queue) = state.queue.lock() {
            queue.push(action, vec![window.label().into()]);
        }
        let _ = window.emit("qingjian-desktop-actions", ());
    }
}
pub fn window_closed(window: &tauri::Window) {
    let state = window.state::<DesktopRuntime>();
    let cancelled = state
        .queue
        .lock()
        .map(|mut queue| queue.window_closed(window.label()))
        .unwrap_or(false);
    if cancelled {
        let _ = window.app_handle().emit("qingjian-desktop-resume", ());
    }
}
#[tauri::command]
pub fn desktop_pending(
    window: tauri::WebviewWindow,
    state: tauri::State<DesktopRuntime>,
) -> Result<Vec<Value>, String> {
    let queue = state
        .queue
        .lock()
        .map_err(|_| "桌面快捷操作暂不可用".to_string())?;
    Ok(queue
        .pending(window.label())
        .into_iter()
        .map(|(id, action)| json!({"id":id,"action":action}))
        .collect())
}
#[tauri::command]
pub fn desktop_complete(
    window: tauri::WebviewWindow,
    state: tauri::State<DesktopRuntime>,
    id: u64,
    success: bool,
) -> Result<bool, String> {
    let (action, effect, participants) = {
        let mut queue = state
            .queue
            .lock()
            .map_err(|_| "桌面快捷操作暂不可用".to_string())?;
        let action = queue.action_for(window.label(), id);
        let participants = queue.participants(id);
        (
            action,
            queue.complete(window.label(), id, success),
            participants,
        )
    };
    if !success {
        show(window.app_handle(), window.label());
        if action.as_deref() == Some("quit") {
            let _ = window.app_handle().emit("qingjian-desktop-resume", ());
        }
    }
    match effect.as_deref() {
        Some("hide") => {
            window
                .hide()
                .map_err(|_| "窗口未能隐藏，请重试".to_string())?;
        }
        Some("close") => {
            window
                .destroy()
                .map_err(|_| "窗口未能关闭，请重试".to_string())?;
        }
        Some("quit") => {
            *state
                .approved
                .lock()
                .map_err(|_| "退出确认未完成".to_string())? = participants;
            state.exiting.store(true, Ordering::SeqCst);
            window.app_handle().exit(0);
        }
        _ => {}
    }
    Ok(success && action.as_deref() == Some("quit"))
}
pub fn exit_requested(app: &tauri::AppHandle, api: &tauri::ExitRequestApi) {
    let state = app.state::<DesktopRuntime>();
    let current: BTreeSet<_> = app.webview_windows().keys().cloned().collect();
    let decision = state
        .approved
        .lock()
        .map(|approved| exit_decision(state.exiting.load(Ordering::SeqCst), &approved, &current))
        .unwrap_or(ExitDecision::Save);
    match decision {
        ExitDecision::Allow => {}
        ExitDecision::Save => {
            api.prevent_exit();
            request(app, "quit");
        }
        ExitDecision::Cancel => {
            api.prevent_exit();
            state.exiting.store(false, Ordering::SeqCst);
            let _ = app.emit("qingjian-desktop-resume", ());
            show(app, "main");
        }
    }
}
#[tauri::command]
pub fn desktop_locale(app: tauri::AppHandle, locale: String) -> Result<(), String> {
    install(&app, &locale).map_err(|_| "系统菜单未能更新".to_string())
}
