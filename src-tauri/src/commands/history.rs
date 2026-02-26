// commands/history.rs - 历史记录管理命令

use tauri::State;
use crate::state::{AppState, HistoryItem};

/// 获取历史记录列表
#[tauri::command]
pub async fn get_history(
    state: State<'_, AppState>,
) -> Result<Vec<HistoryItem>, String> {
    let inner = state.inner.lock()
        .map_err(|e| format!("获取状态锁失败: {}", e))?;

    Ok(inner.history.clone())
}

/// 清空所有历史记录
#[tauri::command]
pub async fn clear_history(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut inner = state.inner.lock()
        .map_err(|e| format!("获取状态锁失败: {}", e))?;

    inner.history.clear();
    log::info!("历史记录已清空");
    Ok(())
}

/// 删除单条历史记录
/// id: 要删除的记录的 ID
#[tauri::command]
pub async fn delete_history_item(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut inner = state.inner.lock()
        .map_err(|e| format!("获取状态锁失败: {}", e))?;

    let before = inner.history.len();
    inner.history.retain(|item| item.id != id);
    let after = inner.history.len();

    if before == after {
        return Err(format!("未找到 ID 为 {} 的记录", id));
    }

    log::info!("已删除历史记录: {}", id);
    Ok(())
}
