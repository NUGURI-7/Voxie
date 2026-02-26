// commands/settings.rs - 应用设置命令（含磁盘持久化）

use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;
use crate::state::{AppState, AppSettings};

const STORE_FILE: &str = "voxie-settings.json";
const STORE_KEY:  &str = "settings";

/// 获取当前设置
#[tauri::command]
pub async fn get_settings(
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let inner = state.inner.lock()
        .map_err(|e| format!("获取状态锁失败: {}", e))?;
    Ok(inner.settings.clone())
}

/// 保存设置到内存 + 磁盘
/// 前端调用：invoke('save_settings', { settings: {...} })
#[tauri::command]
pub async fn save_settings(
    app:      AppHandle,
    settings: AppSettings,
    state:    State<'_, AppState>,
) -> Result<(), String> {
    // 1. 更新内存
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("获取状态锁失败: {}", e))?;
        inner.settings = settings.clone();
    }

    // 2. 持久化到 JSON 文件（tauri-plugin-store 存入 app 数据目录）
    let store = app.store(STORE_FILE)
        .map_err(|e| format!("打开存储失败: {}", e))?;

    let val = serde_json::to_value(&settings)
        .map_err(|e| format!("序列化设置失败: {}", e))?;

    store.set(STORE_KEY, val);
    store.save()
        .map_err(|e| format!("写入磁盘失败: {}", e))?;

    log::info!("设置已持久化到磁盘 ({})", STORE_FILE);
    Ok(())
}

/// 应用启动时从磁盘加载持久化设置，写入 AppState
/// 由 lib.rs setup() 调用
pub fn load_persisted_settings(app: &AppHandle, state: &AppState) {
    let store = match app.store(STORE_FILE) {
        Ok(s)  => s,
        Err(e) => {
            log::info!("首次启动，无持久化文件（{}）", e);
            return;
        }
    };

    let val = match store.get(STORE_KEY) {
        Some(v) => v,
        None    => {
            log::info!("存储文件中尚无 settings 键，使用默认值");
            return;
        }
    };

    match serde_json::from_value::<AppSettings>(val) {
        Ok(settings) => {
            let mut inner = state.inner.lock().unwrap();
            inner.settings = settings;
            log::info!("已从磁盘加载持久化设置");
        }
        Err(e) => {
            log::warn!("持久化设置格式不兼容，使用默认值（{}）", e);
        }
    }
}
