// commands/audio.rs - 录音相关的 Tauri 命令

use tauri::State;
use serde::Serialize;
use crate::state::{AppState, RecordingStatus};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingStatusResponse {
    pub status: RecordingStatus,
    pub duration_ms: u64,
    pub sample_count: usize,
}

/// 开始录音
///
/// 流程：
/// 1. 检查当前不在录音 → 防止重复开始
/// 2. 更新 inner 状态为 Recording，清空旧缓冲区
/// 3. 启动 cpal 音频流（数据会持续写入 recorder 内部的 Arc<Mutex<Vec<f32>>>）
///
/// 关键 Rust 规则：标准 Mutex 的 guard 不能跨越 .await 点
/// 所以每次拿锁都在独立的块 { } 里，用完立即 drop
#[tauri::command]
pub async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    // ---- 第一步：检查并更新业务状态 ----
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;

        if inner.recording_status == RecordingStatus::Recording {
            return Err("已在录音中".to_string());
        }
        inner.recording_status = RecordingStatus::Recording;
        inner.audio_buffer = None; // 清空上次录音数据
    } // ← 锁在这里自动释放，不跨越 await

    // ---- 第二步：启动 cpal 录音流 ----
    {
        let mut recorder = state.recorder.lock()
            .map_err(|e| format!("录音器锁失败: {}", e))?;

        if let Err(e) = recorder.start() {
            // 启动失败，把状态回滚为 Idle
            if let Ok(mut inner) = state.inner.lock() {
                inner.recording_status = RecordingStatus::Idle;
            }
            return Err(format!("启动录音失败: {}", e));
        }
    }

    log::info!("cpal 录音流已启动");
    Ok(())
}

/// 停止录音
///
/// 流程：
/// 1. 停止 cpal 流 → 取回 Vec<f32> PCM 数据
/// 2. 将数据存入 inner.audio_buffer，供 transcribe_audio 消费
/// 3. 状态改为 Processing
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<StopRecordingResponse, String> {
    // ---- 第一步：检查状态 ----
    {
        let inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        if inner.recording_status != RecordingStatus::Recording {
            return Err("当前未在录音".to_string());
        }
    }

    // ---- 第二步：停止录音，取回 PCM 数据 ----
    // stop() 会 drop cpal::Stream（停止采集），返回缓冲区数据
    let audio_data: Vec<f32> = {
        let mut recorder = state.recorder.lock()
            .map_err(|e| format!("录音器锁失败: {}", e))?;
        recorder.stop()
    };

    let sample_count = audio_data.len();
    let duration_ms = (sample_count as f64 / 16000.0 * 1000.0) as u64;

    log::info!("录音停止，采集 {} 样本，{} ms", sample_count, duration_ms);

    // ---- 第三步：存数据，更新状态 ----
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        inner.audio_buffer = Some(audio_data);
        inner.recording_status = RecordingStatus::Processing;
    }

    Ok(StopRecordingResponse { sample_count, duration_ms })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopRecordingResponse {
    pub sample_count: usize,
    pub duration_ms: u64,
}

/// 查询当前录音状态
#[tauri::command]
pub async fn get_recording_status(
    state: State<'_, AppState>,
) -> Result<RecordingStatusResponse, String> {
    let inner = state.inner.lock()
        .map_err(|e| format!("状态锁失败: {}", e))?;

    let sample_count = inner.audio_buffer.as_ref().map(|b| b.len()).unwrap_or(0);

    Ok(RecordingStatusResponse {
        status: inner.recording_status.clone(),
        duration_ms: (sample_count as f64 / 16000.0 * 1000.0) as u64,
        sample_count,
    })
}
