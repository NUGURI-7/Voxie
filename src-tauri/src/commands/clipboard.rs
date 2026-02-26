// commands/clipboard.rs - 剪贴板操作命令

use tauri_plugin_clipboard_manager::ClipboardExt;

/// 将文本复制到剪贴板
/// 前端调用：invoke('copy_to_clipboard', { text: '...' })
#[tauri::command]
pub async fn copy_to_clipboard(
    text: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 使用 tauri-plugin-clipboard-manager 插件
    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| format!("复制到剪贴板失败: {}", e))?;

    log::info!("已复制到剪贴板: {} 个字符", text.len());
    Ok(())
}
