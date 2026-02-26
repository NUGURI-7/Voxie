// state/mod.rs - 全局应用状态管理
// 使用 Arc<Mutex<...>> 保证线程安全
// Arc = 原子引用计数（允许多线程共享所有权）
// Mutex = 互斥锁（同一时间只允许一个线程访问）

use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::audio::AudioRecorder;
use crate::whisper::WhisperEngine;

// ===== 录音状态 =====

/// 录音器当前状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum RecordingStatus {
    Idle,        // 空闲，未录音
    Recording,   // 正在录音
    Processing,  // 正在处理（识别中）
}

impl Default for RecordingStatus {
    fn default() -> Self { RecordingStatus::Idle }
}

// ===== 模型状态 =====

/// Whisper 模型加载状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ModelStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    Loading,
    Ready,
    Error(String),
}

impl Default for ModelStatus {
    fn default() -> Self { ModelStatus::NotDownloaded }
}

// ===== 识别模式 =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptionMode {
    Local,
    Cloud,
}

impl Default for TranscriptionMode {
    fn default() -> Self { TranscriptionMode::Local }
}

// ===== 历史记录 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub mode: TranscriptionMode,
    pub model_name: Option<String>,
}

// ===== 云端服务商 =====

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CloudProvider {
    OpenAI,
    VolcEngine,  // 火山引擎
    Aliyun,      // 阿里云
    Xunfei,      // 讯飞
    Custom,
}

impl Default for CloudProvider {
    fn default() -> Self { CloudProvider::OpenAI }
}

// ===== 应用设置 =====

fn default_theme() -> String { "green".to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub mode: TranscriptionMode,
    pub local_model: String,
    pub cloud_provider: CloudProvider,
    pub cloud_base_url: String,
    pub cloud_api_key: String,
    pub shortcut_key: String,
    pub language: String,
    pub window_opacity: f64,
    pub auto_copy: bool,
    pub max_history: usize,
    /// 主题色：green | blue | violet | ember
    #[serde(default = "default_theme")]
    pub theme: String,
    /// MyMemory 翻译 API Key（可选，留空免费 1000次/天，填入后 10000次/天）
    #[serde(default)]
    pub my_memory_key: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            mode: TranscriptionMode::Local,
            local_model: "small".to_string(),
            cloud_provider: CloudProvider::OpenAI,
            cloud_base_url: "https://api.openai.com/v1".to_string(),
            cloud_api_key: String::new(),
            shortcut_key: "Alt".to_string(),
            language: "auto".to_string(),
            window_opacity: 0.85,
            auto_copy: true,
            max_history: 100,
            theme: "green".to_string(),
            my_memory_key: String::new(),
        }
    }
}

// ===== 内部状态（被单个 Mutex 保护）=====

pub struct InnerState {
    pub recording_status: RecordingStatus,
    pub model_status: ModelStatus,
    pub settings: AppSettings,
    pub history: Vec<HistoryItem>,
    pub download_progress: f64,
    /// 录音完成后保存在这里，等待推理消费
    pub audio_buffer: Option<Vec<f32>>,
    /// 今日翻译已用次数（MyMemory API，无 Key 时本地估算）
    pub translation_day_count: u32,
    /// 计数对应的日期（"2024-02-26"），日期变化时自动归零
    pub translation_day_date: String,
}

impl InnerState {
    pub fn new() -> Self {
        InnerState {
            recording_status: RecordingStatus::default(),
            model_status: ModelStatus::default(),
            settings: AppSettings::default(),
            history: Vec::new(),
            download_progress: 0.0,
            audio_buffer: None,
            translation_day_count: 0,
            translation_day_date: String::new(),
        }
    }
}

// ===== 全局应用状态 =====

/// AppState 由 Tauri 托管，可在所有 #[tauri::command] 中通过 State<AppState> 注入
///
/// 设计要点：
/// - inner：保存业务数据，用 std::sync::Mutex（不是 tokio::Mutex）
///   原因：Tauri 命令在 tokio 线程上运行，但我们在 .await 前后需要
///   保证不跨越 await 持锁，所以用标准 Mutex 即可
/// - recorder：单独存放，避免持锁时间过长（录音流是长生命周期对象）
/// - whisper：单独存放，模型加载/推理是耗时 blocking 操作，
///   放入独立锁 + spawn_blocking 线程，避免阻塞 tokio 运行时
pub struct AppState {
    pub inner: Arc<Mutex<InnerState>>,
    /// 独立的录音器锁，与 inner 分开，防止死锁
    pub recorder: Arc<Mutex<AudioRecorder>>,
    /// Whisper 推理引擎，与 inner 分开，推理期间不阻塞状态读写
    pub whisper: Arc<Mutex<WhisperEngine>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            inner: Arc::new(Mutex::new(InnerState::new())),
            recorder: Arc::new(Mutex::new(AudioRecorder::new())),
            whisper: Arc::new(Mutex::new(WhisperEngine::new())),
        }
    }
}

// cpal::Stream 内部已是 Send，AudioRecorder 也标记了 Send
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}
