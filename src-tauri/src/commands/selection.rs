// commands/selection.rs - 「翻译选中文字」后台监听
//
// macOS：通过 AXUIElement Accessibility API 直接读取选中文字
//        不模拟任何按键，不修改剪贴板
//        需要「辅助功能」权限（系统设置 → 隐私与安全性 → 辅助功能）
//
// Windows / Linux：轮询剪贴板变化
//        用户选中文字后手动 Ctrl+C，Voxie 自动捕获
//        无需额外权限
//
// 共同流程（每 400ms）：
//   功能已开启 && Voxie 未聚焦
//     → 读取当前文字（macOS: AXSelectedText，其他: 剪贴板）
//     → 与上次比对：有变化且 ≥2 字符 → emit "translate-selection" 事件

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_clipboard_manager::ClipboardExt;

// ===== macOS：AXUIElement Accessibility API =====

#[cfg(target_os = "macos")]
mod ax {
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_int, c_void};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> *mut c_void;
        fn AXUIElementCopyAttributeValue(
            element:   *const c_void,
            attribute: *const c_void,
            value:     *mut *mut c_void,
        ) -> c_int;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *const c_void);
        fn CFStringGetLength(s: *const c_void) -> isize;
        fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
        fn CFStringGetCString(
            s:           *const c_void,
            buffer:      *mut c_char,
            buffer_size: isize,
            encoding:    u32,
        ) -> u8;
        fn CFStringCreateWithCString(
            alloc:    *const c_void,
            c_str:    *const c_char,
            encoding: u32,
        ) -> *mut c_void;
    }

    const UTF8: u32 = 0x0800_0100;

    /// 将 Rust &str 转为临时 CFStringRef（调用方负责 CFRelease）
    unsafe fn to_cf_str(s: &str) -> Option<*mut c_void> {
        let c = CString::new(s).ok()?;
        let ptr = CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), UTF8);
        if ptr.is_null() { None } else { Some(ptr) }
    }

    /// CFStringRef → Rust String
    unsafe fn from_cf_str(ptr: *const c_void) -> Option<String> {
        if ptr.is_null() { return None; }
        let len = CFStringGetLength(ptr);
        if len == 0 { return Some(String::new()); }
        let max = CFStringGetMaximumSizeForEncoding(len, UTF8) + 1;
        let mut buf: Vec<c_char> = vec![0; max as usize];
        if CFStringGetCString(ptr, buf.as_mut_ptr(), max, UTF8) == 0 {
            return None;
        }
        CStr::from_ptr(buf.as_ptr())
            .to_str()
            .ok()
            .map(|s| s.to_string())
    }

    /// 读取当前聚焦 UI 元素的选中文字。
    /// 返回 None：无权限 / 无选区 / 不支持的应用。
    pub fn get_selected_text() -> Option<String> {
        unsafe {
            // 系统级 AX 元素 → 当前聚焦元素
            let system = AXUIElementCreateSystemWide();
            if system.is_null() { return None; }

            let focused_key = to_cf_str("AXFocusedUIElement")?;
            let mut focused: *mut c_void = std::ptr::null_mut();
            let err = AXUIElementCopyAttributeValue(
                system as *const c_void,
                focused_key as *const c_void,
                &mut focused,
            );
            CFRelease(system as *const c_void);
            CFRelease(focused_key as *const c_void);
            if err != 0 || focused.is_null() { return None; }

            // 当前聚焦元素 → 选中文字
            let sel_key = to_cf_str("AXSelectedText")?;
            let mut text_ref: *mut c_void = std::ptr::null_mut();
            let err = AXUIElementCopyAttributeValue(
                focused as *const c_void,
                sel_key as *const c_void,
                &mut text_ref,
            );
            CFRelease(focused as *const c_void);
            CFRelease(sel_key as *const c_void);
            if err != 0 || text_ref.is_null() { return None; }

            let result = from_cf_str(text_ref as *const c_void);
            CFRelease(text_ref as *const c_void);
            result
        }
    }
}

// ===== 主监听循环 =====

/// 启动后台监听，只在 app setup 时调用一次。
pub fn spawn_selection_monitor(
    app:            AppHandle,
    active:         Arc<AtomicBool>,
    window_focused: Arc<AtomicBool>,
) {
    tauri::async_runtime::spawn(async move {
        // 记录上次已捕获的文字，防止重复触发
        let mut last = String::new();

        loop {
            tokio::time::sleep(Duration::from_millis(400)).await;

            // 功能关闭 或 Voxie 处于焦点时，跳过
            if !active.load(Ordering::Relaxed)
                || window_focused.load(Ordering::Relaxed)
            {
                continue;
            }

            // 读取当前文字
            let current = read_current_text(&app);
            let trimmed = current.trim().to_string();

            // 过滤：有变化 + 非空 + ≥2 字符
            if !trimmed.is_empty()
                && trimmed != last
                && trimmed.chars().count() >= 2
            {
                last = trimmed.clone();
                log::debug!("[Voxie] 捕获文字 {} 字符", trimmed.chars().count());
                app.emit("translate-selection", trimmed).ok();
            }
        }
    });
}

/// 平台分发：macOS 读 AX 选中文字，其他平台读剪贴板
fn read_current_text(_app: &AppHandle) -> String {
    #[cfg(target_os = "macos")]
    {
        ax::get_selected_text().unwrap_or_default()
    }

    #[cfg(not(target_os = "macos"))]
    {
        _app.clipboard().read_text().unwrap_or_default()
    }
}
