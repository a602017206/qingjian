#[cfg(all(feature = "credential-smoke", target_os = "android"))]
mod credential_smoke;
mod credentials;
#[cfg(desktop)]
mod desktop;
#[cfg(desktop)]
mod desktop_actions;
use notebook_core::Store;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use tauri::Manager;
struct SharedStore(Arc<Mutex<Store>>);

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
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let root = match std::env::var("QINGJIAN_DATA_DIR") {
                Ok(p) => std::path::PathBuf::from(p),
                Err(_) => app.path().app_data_dir()?,
            };
            let mut store = Store::new(&root).map_err(std::io::Error::other)?;
            store.set_credential_vault(Arc::new(credentials::SystemVault::new()));
            notebook_core::configure_network(&root).map_err(std::io::Error::other)?;
            #[cfg(desktop)]
            {
                app.manage(desktop::DesktopRuntime::default());
                let config = store
                    .dispatch("bootstrap", serde_json::json!({}))
                    .map_err(std::io::Error::other)?;
                desktop::install(
                    app.handle(),
                    config["preferences"]["language"]
                        .as_str()
                        .unwrap_or("zh-CN"),
                )?;
            }
            app.manage(SharedStore(Arc::new(Mutex::new(store))));
            #[cfg(all(feature = "credential-smoke", target_os = "android"))]
            tauri::async_runtime::spawn_blocking(move || credential_smoke::run(&root));
            Ok(())
        });
    #[cfg(desktop)]
    let builder = builder
        .on_menu_event(|app, event| desktop::menu_event(app, event.id.as_ref()))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                desktop::close_requested(window, api);
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                desktop::window_closed(window);
            }
        })
        .invoke_handler(tauri::generate_handler![
            dispatch,
            desktop::desktop_pending,
            desktop::desktop_complete,
            desktop::desktop_locale
        ]);
    #[cfg(mobile)]
    let builder = builder.invoke_handler(tauri::generate_handler![dispatch]);
    let app = builder
        .build(tauri::generate_context!())
        .expect("轻笺启动失败");
    app.run(|app, event| {
        #[cfg(desktop)]
        if let tauri::RunEvent::ExitRequested { ref api, .. } = event {
            desktop::exit_requested(app, api);
        }
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = event {
            desktop::show(app, "main");
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (app, event);
    });
}
