// tray/mod.rs - 系统托盘模块
// 实现菜单栏图标和托盘菜单

use tauri::{
    App, Manager, Emitter,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use anyhow::Result;

/// 设置系统托盘
/// 在 macOS 上显示在菜单栏，Dock 栏不显示图标
pub fn setup_tray(app: &mut App) -> Result<()> {
    // 创建托盘菜单项
    // MenuItem::with_id 创建带有唯一 ID 的菜单项，
    // ID 用于在菜单事件回调中识别用户点击了哪一项

    // "打开/隐藏悬浮窗" 菜单项
    let toggle_item = MenuItem::with_id(
        app,
        "toggle_window",
        "显示/隐藏悬浮窗",
        true,   // 是否可点击
        None::<&str>,  // 快捷键（None 表示无）
    )?;

    // 分隔线
    let separator = PredefinedMenuItem::separator(app)?;

    // "设置" 菜单项
    let settings_item = MenuItem::with_id(
        app,
        "open_settings",
        "设置...",
        true,
        None::<&str>,
    )?;

    // 另一条分隔线
    let separator2 = PredefinedMenuItem::separator(app)?;

    // "退出" 菜单项
    let quit_item = MenuItem::with_id(
        app,
        "quit",
        "退出 Voxie",
        true,
        None::<&str>,
    )?;

    // 组装菜单
    let menu = Menu::with_items(
        app,
        &[
            &toggle_item,
            &separator,
            &settings_item,
            &separator2,
            &quit_item,
        ],
    )?;

    // 构建托盘图标
    // 使用叶子 V 形的专属托盘图标（黑色 + 透明背景，macOS template image）
    let tray_icon = tauri::image::Image::from_bytes(
        include_bytes!("../../icons/tray-icon.png")
    ).expect("无法加载托盘图标");

    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)   // macOS：让系统自动适配深/浅色菜单栏
        .menu(&menu)
        // 菜单事件处理：用户点击菜单项时触发
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "toggle_window" => {
                    // 切换悬浮窗的显示/隐藏状态
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
                "open_settings" => {
                    // 显示悬浮窗并导航到设置页
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        // 通过发送事件到前端，让 Vue Router 导航到设置页
                        let _ = window.emit("navigate-to-settings", ());
                    }
                }
                "quit" => {
                    // 退出应用
                    app.exit(0);
                }
                _ => {
                    log::warn!("未知的托盘菜单事件: {}", event.id.as_ref());
                }
            }
        })
        // 点击托盘图标本身时（不是菜单项）
        .on_tray_icon_event(|tray, event| {
            // 在 macOS 上，单击托盘图标通常显示菜单
            // Tauri 会自动处理这个行为
            use tauri::tray::TrayIconEvent;
            if let TrayIconEvent::Click { .. } = event {
                // 可以在这里处理单击事件
            }
        })
        .build(app)?;

    log::info!("系统托盘初始化完成");
    Ok(())
}
