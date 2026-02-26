// lib.rs - Rust 侧主库入口
// 整个应用的逻辑核心，被 main.rs 调用

// 声明各个子模块
pub mod audio;      // 音频录制模块
pub mod whisper;    // 本地 Whisper 推理模块
pub mod cloud;      // 云端 API 调用模块
pub mod commands;   // Tauri 命令（前端通过 invoke 调用）
pub mod state;      // 全局应用状态
pub mod tray;       // 系统托盘

use tauri::Manager;

/// 应用程序主入口函数
/// 由 main.rs 调用
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志系统
    // 调试模式：显示所有日志
    // 发布模式：显示 voxie 模块的 info 日志（用于诊断 Whisper 等运行时问题）
    #[cfg(debug_assertions)]
    std::env::set_var("RUST_LOG", "voxie=debug,tauri=info");
    #[cfg(not(debug_assertions))]
    std::env::set_var("RUST_LOG", "voxie=info,tauri=warn");

    env_logger::init();

    log::info!("Voxie 启动中...");

    // 构建 Tauri 应用
    tauri::Builder::default()
        // ===== 注册插件 =====
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        // ===== 注册全局应用状态 =====
        // 这里注册的状态可以在所有 Tauri 命令中通过参数注入获取
        .manage(state::AppState::new())
        // ===== 应用启动时的初始化逻辑 =====
        .setup(|app| {
            log::info!("应用 setup 开始");

            // ── 从磁盘加载持久化设置 ──
            {
                let app_state = app.state::<state::AppState>();
                commands::settings::load_persisted_settings(
                    app.handle(),
                    &app_state,
                );
            }

            // 初始化系统托盘
            tray::setup_tray(app)?;

            // 注册全局快捷键（默认右 Option 键）
            // 注意：全局快捷键在这里只是初始化框架，
            // 实际的监听逻辑由前端配置后通过 command 注册
            log::info!("全局快捷键框架初始化完成");

            // 获取主窗口并配置
            if let Some(window) = app.get_webview_window("main") {
                // macOS 特有：设置窗口始终置顶
                #[cfg(target_os = "macos")]
                {
                    window.set_always_on_top(true)?;
                    log::info!("悬浮窗置顶设置完成");
                }
            }

            log::info!("应用初始化完成");
            Ok(())
        })
        // ===== 注册 Tauri 命令 =====
        // 前端通过 invoke('command_name', args) 调用这些函数
        .invoke_handler(tauri::generate_handler![
            // 录音相关命令
            commands::audio::start_recording,
            commands::audio::stop_recording,
            commands::audio::get_recording_status,
            // 识别相关命令
            commands::transcribe::transcribe_audio,
            commands::transcribe::get_transcription_status,
            commands::transcribe::test_cloud_connection,
            // 翻译命令
            commands::translate::translate_text,
            commands::translate::get_translation_usage,
            // 模型管理命令
            commands::model::download_model,
            commands::model::load_whisper_model,
            commands::model::unload_whisper_model,
            commands::model::get_model_status,
            commands::model::list_models,
            commands::model::delete_model,
            // 设置命令
            commands::settings::get_settings,
            commands::settings::save_settings,
            // 历史记录命令
            commands::history::get_history,
            commands::history::clear_history,
            commands::history::delete_history_item,
            // 剪贴板命令
            commands::clipboard::copy_to_clipboard,
            // 窗口命令
            commands::window::toggle_window_visibility,
            commands::window::set_window_opacity,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri 应用启动失败");
}
