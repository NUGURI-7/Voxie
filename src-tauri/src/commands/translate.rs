// commands/translate.rs - 文本翻译命令
//
// 翻译方案：
//   简体 ↔ 繁体  →  zhconv（本地，纯 Rust，零 API 调用）
//   其他方向     →  MyMemory 免费翻译 API（无需注册，1000次/天；填入 Key 后 10000次/天）

use tauri::State;
use serde::Serialize;
use zhconv::{zhconv, Variant};
use chrono::Local;
use crate::state::AppState;

// ===== 语言代码映射 =====

/// 内部语言码 → MyMemory langpair 组成部分
fn to_mm_lang(lang: &str) -> &str {
    match lang {
        "zh-hans" => "zh-CN",
        "zh-hant" => "zh-TW",
        "en"      => "en-GB",
        _         => lang,
    }
}

// ===== 翻译用量响应 =====

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationUsage {
    /// 今日已用次数
    pub used_today: u32,
    /// 今日限额（无 Key = 1000，有 Key = 10000）
    pub limit_today: u32,
    /// 是否配置了 Key
    pub has_key: bool,
}

// ===== 翻译命令 =====

/// 翻译文本
///
/// - from / to: "zh-hans" | "zh-hant" | "en"
/// - 简↔繁 使用 zhconv 本地完成，不消耗 API 额度
/// - 其他方向调用 MyMemory API
#[tauri::command]
pub async fn translate_text(
    text: String,
    from: String,
    to: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // 相同语言：直接返回
    if from == to || text.trim().is_empty() {
        return Ok(text);
    }

    // ── 简 → 繁（本地）──
    if from == "zh-hans" && to == "zh-hant" {
        return Ok(zhconv(&text, Variant::ZhHant));
    }
    // ── 繁 → 简（本地）──
    if from == "zh-hant" && to == "zh-hans" {
        return Ok(zhconv(&text, Variant::ZhHans));
    }

    // ── 其他方向：MyMemory API ──
    let api_key = {
        let inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        inner.settings.my_memory_key.clone()
    };

    let langpair = format!("{}|{}", to_mm_lang(&from), to_mm_lang(&to));

    let client = reqwest::Client::new();
    let mut req = client
        .get("https://api.mymemory.translated.net/get")
        .query(&[("q", text.as_str()), ("langpair", langpair.as_str())]);

    if !api_key.is_empty() {
        req = req.query(&[("key", api_key.as_str())]);
    }

    let resp = req.send().await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    let json: serde_json::Value = resp.json().await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    // 检查限额
    let quota_finished = json["quotaFinished"].as_bool().unwrap_or(false);
    let translated = json["responseData"]["translatedText"]
        .as_str()
        .unwrap_or("");

    if quota_finished || translated.starts_with("MYMEMORY WARNING:") {
        return Err("今日翻译次数已用完，请明天再试，或在设置中填入 MyMemory Key 提升至 10000次/天".to_string());
    }

    if translated.is_empty() {
        return Err(format!("翻译失败，服务器响应: {}", json));
    }

    // 更新今日计数
    {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        if inner.translation_day_date != today {
            inner.translation_day_count = 0;
            inner.translation_day_date = today;
        }
        inner.translation_day_count += 1;
    }

    Ok(translated.to_string())
}

// ===== 用量查询命令 =====

/// 查询今日翻译用量
///
/// - 无 Key：返回本地计数（估算），限额 1000
/// - 有 Key：尝试查询 MyMemory /usage 接口获取精确值，失败则退回本地计数
#[tauri::command]
pub async fn get_translation_usage(
    state: State<'_, AppState>,
) -> Result<TranslationUsage, String> {
    let (api_key, local_count) = {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        // 日期变化时归零
        let today = Local::now().format("%Y-%m-%d").to_string();
        if inner.translation_day_date != today {
            inner.translation_day_count = 0;
            inner.translation_day_date = today;
        }
        (inner.settings.my_memory_key.clone(), inner.translation_day_count)
    };

    let has_key = !api_key.is_empty();

    // 有 Key 时尝试获取精确用量
    if has_key {
        let client = reqwest::Client::new();
        if let Ok(resp) = client
            .get("https://api.mymemory.translated.net/usage")
            .query(&[("key", api_key.as_str())])
            .send().await
        {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                // MyMemory /usage 返回格式：{"status":200,"data":{"usage":{...}}}
                // day_requests 字段为当日请求数
                if let Some(used) = json["data"]["usage"]["day_requests"].as_u64() {
                    return Ok(TranslationUsage {
                        used_today: used as u32,
                        limit_today: 10_000,
                        has_key: true,
                    });
                }
            }
        }
        // API 查询失败：退回本地计数
        return Ok(TranslationUsage {
            used_today: local_count,
            limit_today: 10_000,
            has_key: true,
        });
    }

    Ok(TranslationUsage {
        used_today: local_count,
        limit_today: 1_000,
        has_key: false,
    })
}
