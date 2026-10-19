//! 仅云端测试包启用，正式安装包不包含此模块。
use crate::credentials::SystemVault;
use notebook_core::{CredentialVault, Result};
use std::{ffi::CString, path::Path};

fn report(marker: &str) {
    #[link(name = "log")]
    extern "C" {
        fn __android_log_write(
            priority: i32,
            tag: *const std::ffi::c_char,
            text: *const std::ffi::c_char,
        ) -> i32;
    }
    let tag = CString::new("QingjianCredentialSmoke").unwrap();
    let text = CString::new(marker).unwrap();
    unsafe {
        __android_log_write(4, tag.as_ptr(), text.as_ptr());
    }
}
fn verify(root: &Path) -> Result<&'static str> {
    let state = root.join("credential-smoke-stage.txt");
    let stage = if state.exists() {
        std::fs::read_to_string(&state).map_err(|_| "无法读取测试阶段")?
    } else {
        "0".into()
    };
    let vault = SystemVault::new();
    let key = "ci-android-credential-persistence";
    let expected = (
        "anonymous-ci-user".to_owned(),
        "anonymous-ci-token".to_owned(),
    );
    let marker = match stage.trim() {
        "0" => {
            vault.set(key, &expected.0, &expected.1)?;
            if vault.get(key)? != Some(expected) {
                return Err("安卓安全认证保存未生效".into());
            }
            std::fs::write(&state, "1").map_err(|_| "无法保存测试阶段")?;
            "SAVED"
        }
        "1" => {
            if vault.get(key)? != Some(expected) {
                return Err("进程重启后安卓安全认证丢失".into());
            }
            vault.delete(key)?;
            if vault.get(key)?.is_some() {
                return Err("安卓安全认证未被删除".into());
            }
            std::fs::write(&state, "2").map_err(|_| "无法保存测试阶段")?;
            "RESTORED_AND_DELETED"
        }
        "2" => {
            if vault.get(key)?.is_some() {
                return Err("已忘记的安卓安全认证在重启后恢复".into());
            }
            "FORGOTTEN"
        }
        _ => return Err("安卓认证测试阶段无效".into()),
    };
    Ok(marker)
}
pub fn run(root: &Path) {
    match verify(root) {
        Ok(marker) => report(marker),
        Err(_) => report("FAILED"),
    }
}
