// commands/window.rs - 窗口控制命令

use tauri::{Manager, Emitter};

/// 切换悬浮窗的显示/隐藏
#[tauri::command]
pub async fn toggle_window_visibility(
    app: tauri::AppHandle,
) -> Result<bool, String> {
    if let Some(window) = app.get_webview_window("main") {
        let is_visible = window.is_visible()
            .map_err(|e| format!("获取窗口状态失败: {}", e))?;

        if is_visible {
            window.hide().map_err(|e| format!("隐藏窗口失败: {}", e))?;
            Ok(false)
        } else {
            window.show().map_err(|e| format!("显示窗口失败: {}", e))?;
            // macOS：show() 后重新断言置顶，防止窗口层级被重置
            #[cfg(target_os = "macos")]
            let _ = window.set_always_on_top(true);
            Ok(true)
        }
    } else {
        Err("未找到主窗口".to_string())
    }
}

/// 设置窗口透明度
/// opacity: 0.0（完全透明）到 1.0（完全不透明）
#[tauri::command]
pub async fn set_window_opacity(
    opacity: f64,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 确保透明度在有效范围内
    let opacity = opacity.clamp(0.1, 1.0);

    if let Some(window) = app.get_webview_window("main") {
        // Tauri 2 目前通过 CSS 处理透明度
        // 通过向前端发送事件来更新 CSS 变量
        let _ = window.emit("update-opacity", opacity);
        log::info!("窗口透明度更新: {}", opacity);
        Ok(())
    } else {
        Err("未找到主窗口".to_string())
    }
}
