// commands/mod.rs - Tauri 命令模块入口
// 所有通过 invoke() 从前端调用的 Rust 函数都在这里

pub mod audio;
pub mod transcribe;
pub mod model;
pub mod translate;
pub mod settings;
pub mod history;
pub mod clipboard;
pub mod window;
