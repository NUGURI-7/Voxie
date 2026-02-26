// main.rs - 应用程序入口点
// 注意：Tauri 要求 main.rs 非常简洁，实际逻辑放在 lib.rs 中
// 这样可以支持热重载和 iOS/Android 等平台

// 在 Windows 发布版本中，禁止弹出控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 调用 lib.rs 中定义的真正入口函数
    voxie_lib::run()
}
