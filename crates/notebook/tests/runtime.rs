use notebook_core::Store;
use serde_json::json;
#[test]
fn app_info_reports_compiled_version_and_target_without_creating_notes() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::new(temp.path()).unwrap();
    let info = store.dispatch("appInfo", json!({})).unwrap();
    assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(info["os"], std::env::consts::OS);
    assert_eq!(info["arch"], std::env::consts::ARCH);
    assert_eq!(info["simulator"], cfg!(target_abi = "sim"));
    assert_eq!(
        std::fs::read_dir(temp.path().join("workspaces"))
            .unwrap()
            .count(),
        0
    );
}
