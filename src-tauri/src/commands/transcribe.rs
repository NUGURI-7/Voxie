// commands/transcribe.rs - 语音识别相关的 Tauri 命令

use tauri::{State, Emitter};
use serde::Serialize;
use crate::state::{AppState, RecordingStatus, TranscriptionMode, ModelStatus, HistoryItem};
use crate::cloud::{transcribe_cloud, CloudTranscribeParams};

/// Whisper 推理超时时间（秒）
/// Windows 启用 CUDA 后大模型也应在 10 秒内完成，120 秒是安全边界
/// 如果用户没有 NVIDIA 显卡 / 没装 CUDA 驱动，会自动回退 CPU，此时仍有超时保护
const INFERENCE_TIMEOUT_SECS: u64 = 120;

/// 推理线程栈大小：64MB
/// whisper.cpp 使用大量局部变量/递归，Windows 默认 1MB 栈会导致闪退（栈溢出）
/// 64MB 足够所有模型（包括 Large-v3）正常运行
const INFERENCE_STACK_SIZE: usize = 64 * 1024 * 1024;

/// 模型加载线程栈大小：32MB
const LOAD_STACK_SIZE: usize = 32 * 1024 * 1024;

// ===== 识别状态查询 =====

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionStatusResponse {
    pub is_processing: bool,
    pub last_result: Option<String>,
}

#[tauri::command]
pub async fn get_transcription_status(
    state: State<'_, AppState>,
) -> Result<TranscriptionStatusResponse, String> {
    let inner = state.inner.lock()
        .map_err(|e| format!("状态锁失败: {}", e))?;

    Ok(TranscriptionStatusResponse {
        is_processing: inner.recording_status == RecordingStatus::Processing,
        last_result: inner.history.first().map(|h| h.text.clone()),
    })
}

// ===== 核心识别命令 =====

/// 识别命令结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscribeResult {
    pub text: String,
    pub duration_ms: u64,
    pub item_id: String,
}

/// 执行语音识别
///
/// 这是核心命令，由前端在 stop_recording 返回后立即调用。
/// 流程：
/// 1. 从 inner.audio_buffer 取出 PCM 数据（stop_recording 已经放进来了）
/// 2. 根据 settings.mode 选择本地（Whisper）或云端（HTTP API）
/// 3. 把结果写入 history，状态回 Idle
/// 4. 通过 Tauri 事件通知前端更新 UI
///
/// 重要规则：不能在持有 Mutex 锁的同时 .await
/// 所以先拿数据、释放锁，再 await，再拿锁写结果
#[tauri::command]
pub async fn transcribe_audio(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<TranscribeResult, String> {

    // ── 第一步：把需要的数据从 inner 里取出来，然后立即释放锁 ──────────
    let (settings, audio_data, duration_ms) = {
        let inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;

        let audio = inner.audio_buffer.clone().unwrap_or_default();
        if audio.is_empty() {
            return Err("没有录音数据，请先完成录音".to_string());
        }

        let dur = (audio.len() as f64 / 16000.0 * 1000.0) as u64;
        (inner.settings.clone(), audio, dur)
    }; // ← 锁释放，后面可以安全 .await

    log::info!(
        "开始识别：模式={:?}, 音频={}ms, 语言={}",
        settings.mode, duration_ms, settings.language
    );

    // ── 第二步：执行识别（可能耗时很长，所以在锁外 await）─────────────
    let result_text = match &settings.mode {
        TranscriptionMode::Cloud => {
            // 云端 API 调用
            if settings.cloud_api_key.is_empty() {
                return Err("云端模式需要配置 API Key，请到设置页面填写".to_string());
            }
            if settings.cloud_base_url.is_empty() {
                return Err("云端模式需要配置 Base URL，请到设置页面填写".to_string());
            }

            let params = CloudTranscribeParams {
                audio_samples: audio_data,
                language: settings.language.clone(),
                provider: settings.cloud_provider.clone(),
                base_url: settings.cloud_base_url.clone(),
                api_key: settings.cloud_api_key.clone(),
            };

            transcribe_cloud(params)
                .await
                .map_err(|e| format!("云端识别失败: {}", e))?
        }

        TranscriptionMode::Local => {
            // ── 本地 Whisper 推理 ──────────────────────────────────────────

            // 1. 检查模型是否已下载
            let model = crate::whisper::WhisperModel::from_str(&settings.local_model)
                .ok_or_else(|| format!("未知模型 \"{}\"，请到设置页面重新选择", settings.local_model))?;

            if !crate::whisper::is_model_downloaded(&model) {
                return Err(format!(
                    "模型 {} 尚未下载，请先到设置 → 本地模型 页面下载",
                    model.display_name()
                ));
            }

            let model_path = crate::whisper::get_model_path(&model)
                .map_err(|e| format!("获取模型路径失败: {}", e))?;

            // 2. 判断是否需要（重新）加载模型
            //    同一个模型已加载则跳过，换了模型才重新加载
            let needs_load = {
                let eng = state.whisper.lock()
                    .map_err(|e| format!("引擎锁失败: {}", e))?;
                eng.current_model_name().map(|s| s.to_string())
                    != Some(model.filename().to_string())
            };

            if needs_load {
                // 通知前端：正在加载模型
                {
                    let mut inner = state.inner.lock()
                        .map_err(|e| format!("状态锁失败: {}", e))?;
                    inner.model_status = ModelStatus::Loading;
                }

                log::info!("加载 Whisper 模型: {}", model.display_name());

                // 模型加载：使用大栈线程（避免 Windows 1MB 默认栈溢出）
                let whisper_arc = state.whisper.clone();
                let (load_tx, load_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
                std::thread::Builder::new()
                    .name("whisper-model-load".to_string())
                    .stack_size(LOAD_STACK_SIZE)
                    .spawn(move || {
                        let result = (|| -> Result<(), String> {
                            let mut eng = whisper_arc.lock()
                                .map_err(|e| format!("引擎锁失败: {}", e))?;
                            eng.load_model(&model_path)
                                .map_err(|e| format!("加载模型失败: {}", e))
                        })();
                        let _ = load_tx.send(result);
                    })
                    .map_err(|e| format!("创建加载线程失败: {}", e))?;

                load_rx.await
                    .map_err(|e| format!("加载线程通信失败: {}", e))
                    .and_then(|r| r)?;

                // 加载完成，更新状态
                {
                    let mut inner = state.inner.lock()
                        .map_err(|e| format!("状态锁失败: {}", e))?;
                    inner.model_status = ModelStatus::Ready;
                }

                log::info!("模型加载完成: {}", model.display_name());
            }

            // 3. 执行推理（同样是 blocking，放入专用线程）
            //    添加超时保护：Windows CPU 推理可能非常慢
            log::info!(
                "开始本地 Whisper 推理，语言: {}, 超时: {}秒",
                settings.language, INFERENCE_TIMEOUT_SECS
            );

            let whisper_arc = state.whisper.clone();
            let audio_clone = audio_data.clone();
            let lang_clone  = settings.language.clone();

            // 使用 64MB 大栈线程 + oneshot channel：
            // whisper.cpp 推理在 Windows 上需要大量栈空间，
            // 默认 1MB 栈会导致栈溢出闪退（即使是 Tiny 模型）
            let (infer_tx, infer_rx) = tokio::sync::oneshot::channel::<Result<String, String>>();
            std::thread::Builder::new()
                .name("whisper-inference".to_string())
                .stack_size(INFERENCE_STACK_SIZE)
                .spawn(move || {
                    let result = (|| -> Result<String, String> {
                        let eng = whisper_arc.lock()
                            .map_err(|e| format!("引擎锁失败: {}", e))?;
                        eng.transcribe(&audio_clone, &lang_clone)
                            .map_err(|e| format!("本地识别失败: {}", e))
                    })();
                    let _ = infer_tx.send(result);
                })
                .map_err(|e| format!("创建推理线程失败: {}", e))?;

            // 等待推理完成，带超时保护
            let timeout_duration = std::time::Duration::from_secs(INFERENCE_TIMEOUT_SECS);
            match tokio::time::timeout(timeout_duration, infer_rx).await {
                Ok(Ok(result)) => result?,
                Ok(Err(e)) => return Err(format!("推理线程通信失败: {}", e)),
                Err(_elapsed) => {
                    log::error!(
                        "Whisper 推理超时（{}秒），放弃等待",
                        INFERENCE_TIMEOUT_SECS
                    );
                    {
                        let mut inner = state.inner.lock()
                            .map_err(|e| format!("状态锁失败: {}", e))?;
                        inner.recording_status = RecordingStatus::Idle;
                        inner.audio_buffer = None;
                    }
                    return Err(format!(
                        "本地识别超时（已等待 {} 秒）。\n\
                         建议：\n\
                         1. 使用更小的模型（如 Tiny 或 Base）\n\
                         2. 缩短录音时长\n\
                         3. 或切换到云端识别模式",
                        INFERENCE_TIMEOUT_SECS
                    ));
                }
            }
        }
    };

    // ── 第三步：把结果写回 inner，更新历史 ──────────────────────────────
    let item_id = make_id();
    let item = HistoryItem {
        id: item_id.clone(),
        text: result_text.clone(),
        timestamp: chrono::Utc::now(),
        duration_ms,
        mode: settings.mode.clone(),
        model_name: None,
    };

    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;

        inner.history.insert(0, item.clone()); // 最新的排最前

        // 超出上限则截断
        let max = inner.settings.max_history;
        inner.history.truncate(max);

        // 清空缓冲区，状态回 Idle
        inner.audio_buffer = None;
        inner.recording_status = RecordingStatus::Idle;
    }

    // ── 第四步：通知前端 ─────────────────────────────────────────────────
    // emit 是 Tauri 的事件广播，前端通过 listen('new-transcription', ...) 接收
    let _ = app.emit("new-transcription", &item);

    Ok(TranscribeResult {
        text: result_text,
        duration_ms,
        item_id,
    })
}

// ===== 测试云端连接 =====

/// 测试云端 API 是否可用
///
/// 根据 provider 分两条路：
/// - "aliyun" → 调 NLS RESTful 接口（空 body 探测）
/// - 其他      → 调 GET /models（OpenAI 兼容）
#[tauri::command]
pub async fn test_cloud_connection(
    base_url: String,
    api_key: String,
    provider: String,      // 前端传入，如 "aliyun" / "openAI" / ...
) -> Result<String, String> {
    use std::time::Duration;

    // 阿里云走专属 NLS 测试逻辑
    if provider == "aliyun" {
        return crate::cloud::test_aliyun_nls(&base_url, &api_key).await;
    }

    // === OpenAI 兼容服务：GET /models ===
    if base_url.is_empty() {
        return Err("请先填写 Base URL".to_string());
    }
    if api_key.is_empty() {
        return Err("请先填写 API Key".to_string());
    }

    let url = format!("{}/models", base_url.trim_end_matches('/'));
    log::info!("测试云端连接 ({}): {}", provider, url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("dns") || msg.contains("resolve") {
                "域名解析失败，请检查 Base URL".to_string()
            } else if msg.contains("connect") {
                "无法连接到服务器，请检查 Base URL 和网络".to_string()
            } else if msg.contains("timeout") {
                "连接超时，请检查网络或 Base URL".to_string()
            } else {
                format!("请求失败: {}", e)
            }
        })?;

    match resp.status().as_u16() {
        200..=299 => Ok("连接成功".to_string()),
        401 | 403 => Err("API Key 无效或权限不足".to_string()),
        404       => Ok("服务可达（/models 不支持，转写接口通常仍可用）".to_string()),
        429       => Err("请求频率超限，稍后再试".to_string()),
        code      => Err(format!("服务返回异常状态: {}", code)),
    }
}

// ── 工具函数 ────────────────────────────────────────────────────────────────

/// 生成简单唯一 ID（时间戳 + 纳秒，足够在单机上不重复）
fn make_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}{:09}", t.as_secs(), t.subsec_nanos())
}
