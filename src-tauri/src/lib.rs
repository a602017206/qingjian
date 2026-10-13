use notebook_core::Store;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;
struct SharedStore(Arc<Mutex<Store>>);

#[cfg(desktop)]
fn chinese_menu(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, PredefinedMenuItem as Item, Submenu};
    let application = Submenu::with_items(
        app,
        "轻笺",
        true,
        &[
            &Item::about(app, Some("关于轻笺"), None)?,
            &Item::separator(app)?,
            &Item::hide(app, Some("隐藏轻笺"))?,
            &Item::hide_others(app, Some("隐藏其他应用"))?,
            &Item::separator(app)?,
            &Item::quit(app, Some("退出轻笺"))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        "编辑",
        true,
        &[
            &Item::undo(app, Some("撤销"))?,
            &Item::redo(app, Some("重做"))?,
            &Item::separator(app)?,
            &Item::cut(app, Some("剪切"))?,
            &Item::copy(app, Some("复制"))?,
            &Item::paste(app, Some("粘贴"))?,
            &Item::select_all(app, Some("全选"))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        "窗口",
        true,
        &[
            &Item::minimize(app, Some("最小化"))?,
            &Item::maximize(app, Some("放大窗口"))?,
            &Item::fullscreen(app, Some("切换全屏"))?,
            &Item::close_window(app, Some("关闭窗口"))?,
        ],
    )?;
    app.set_menu(Menu::with_items(app, &[&application, &edit, &window])?)?;
    Ok(())
}
#[tauri::command]
async fn dispatch(
    state: tauri::State<'_, SharedStore>,
    action: String,
    request: Value,
) -> Result<Value, String> {
    let store = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        store
            .lock()
            .map_err(|_| "笔记服务暂时不可用".to_owned())?
            .dispatch(&action, request)
    })
    .await
    .map_err(|_| "操作任务未完成".to_owned())?
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            chinese_menu(app)?;
            let root = match std::env::var("QINGJIAN_DATA_DIR") {
                Ok(p) => std::path::PathBuf::from(p),
                Err(_) => app.path().app_data_dir()?,
            };
            let store = Store::new(&root).map_err(std::io::Error::other)?;
            notebook_core::configure_network(&root).map_err(std::io::Error::other)?;
            app.manage(SharedStore(Arc::new(Mutex::new(store))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![dispatch])
        .run(tauri::generate_context!())
        .expect("轻笺启动失败");
}
