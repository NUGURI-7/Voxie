// whisper/mod.rs - 本地 Whisper 语音识别模块
// 使用 whisper-rs 封装 whisper.cpp 的 Rust 绑定
// whisper.cpp 是 Whisper 模型的高性能 C++ 实现

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// 支持的模型大小
/// 模型越大，识别精度越高，但需要更多内存和计算时间
#[derive(Debug, Clone)]
pub enum WhisperModel {
    Tiny,        // ~39M，最快，精度最低
    Base,        // ~74M，快
    Small,       // ~244M，平衡（推荐日常使用）
    Medium,      // ~769M，慢但准确
    LargeV3,     // ~1.5G，最慢最准确
}

impl WhisperModel {
    /// 模型文件名（下载时使用）
    pub fn filename(&self) -> &str {
        match self {
            WhisperModel::Tiny => "ggml-tiny.bin",
            WhisperModel::Base => "ggml-base.bin",
            WhisperModel::Small => "ggml-small.bin",
            WhisperModel::Medium => "ggml-medium.bin",
            WhisperModel::LargeV3 => "ggml-large-v3.bin",
        }
    }

    /// 模型下载 URL（Hugging Face 镜像）
    pub fn download_url(&self) -> String {
        // 使用 Hugging Face 的 ggml 格式模型
        let base = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
        format!("{}/{}", base, self.filename())
    }

    /// 显示名称
    pub fn display_name(&self) -> &str {
        match self {
            WhisperModel::Tiny => "Tiny (~39MB)",
            WhisperModel::Base => "Base (~74MB)",
            WhisperModel::Small => "Small (~244MB)",
            WhisperModel::Medium => "Medium (~769MB)",
            WhisperModel::LargeV3 => "Large-v3 (~1.5GB)",
        }
    }

    /// 从字符串解析模型名
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "tiny" => Some(WhisperModel::Tiny),
            "base" => Some(WhisperModel::Base),
            "small" => Some(WhisperModel::Small),
            "medium" => Some(WhisperModel::Medium),
            "large-v3" | "large_v3" | "largev3" => Some(WhisperModel::LargeV3),
            _ => None,
        }
    }
}

/// 获取模型存储目录
/// macOS/Linux: ~/.local/share/voxie/models/
/// Windows: %LOCALAPPDATA%\voxie\models\
pub fn get_models_dir() -> Result<PathBuf> {
    let base_dir = dirs::data_local_dir()
        .context("无法获取用户数据目录")?;
    let models_dir = base_dir.join("voxie").join("models");

    // 如果目录不存在则创建
    if !models_dir.exists() {
        std::fs::create_dir_all(&models_dir)
            .context("无法创建模型目录")?;
    }

    Ok(models_dir)
}

/// 获取指定模型的文件路径
pub fn get_model_path(model: &WhisperModel) -> Result<PathBuf> {
    let models_dir = get_models_dir()?;
    Ok(models_dir.join(model.filename()))
}

/// 检查模型是否已下载
pub fn is_model_downloaded(model: &WhisperModel) -> bool {
    match get_model_path(model) {
        Ok(path) => path.exists(),
        Err(_) => false,
    }
}

/// 计算音频数据的 RMS 音量（用于检测静音）
pub fn audio_rms(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = data.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum_sq / data.len() as f64).sqrt() as f32
}

/// 获取推荐的线程数
/// Windows CPU 模式下使用物理核心数（避免超线程争用导致挂死）
/// macOS 使用逻辑核心数（Metal GPU 加速为主，CPU 线程影响较小）
fn recommended_threads() -> i32 {
    let physical = num_cpus::get_physical() as i32;
    let logical = num_cpus::get() as i32;

    // Windows CPU-only：使用物理核心数，上限 4
    // 过多线程在 Windows whisper.cpp 上可能导致线程争用
    #[cfg(target_os = "windows")]
    let threads = std::cmp::min(physical, 4);

    // macOS / Linux：使用逻辑核心数，上限 8
    #[cfg(not(target_os = "windows"))]
    let threads = std::cmp::min(logical, 8);

    // 至少 1 个线程
    let threads = std::cmp::max(threads, 1);

    log::info!(
        "CPU: {} 物理核 / {} 逻辑核 → 使用 {} 线程",
        physical, logical, threads
    );
    threads
}

/// Whisper 识别引擎
/// 封装了 WhisperContext 的生命周期管理
pub struct WhisperEngine {
    // WhisperContext 是 whisper.cpp 的主要上下文对象
    // Option 表示可能未初始化（模型未加载）
    ctx: Option<WhisperContext>,
    // 当前加载的模型名称
    current_model: Option<String>,
}

impl WhisperEngine {
    /// 创建新的引擎实例（未加载模型）
    pub fn new() -> Self {
        WhisperEngine {
            ctx: None,
            current_model: None,
        }
    }

    /// 加载 Whisper 模型
    /// model_path: 模型文件的完整路径
    pub fn load_model(&mut self, model_path: &Path) -> Result<()> {
        log::info!("开始加载 Whisper 模型: {:?}", model_path);

        if !model_path.exists() {
            anyhow::bail!("模型文件不存在: {:?}", model_path);
        }

        // 检查文件大小（模型文件损坏或下载不完整的常见表现）
        let file_size = std::fs::metadata(model_path)
            .context("无法读取模型文件信息")?
            .len();
        log::info!("模型文件大小: {:.1} MB", file_size as f64 / 1024.0 / 1024.0);

        if file_size < 1024 * 1024 {
            anyhow::bail!(
                "模型文件过小 ({} bytes)，可能下载不完整，请删除后重新下载",
                file_size
            );
        }

        // 配置 Whisper 上下文参数
        let params = WhisperContextParameters::default();

        // 创建 Whisper 上下文（这一步会加载模型权重到内存）
        // 在 Apple Silicon 上，如果启用了 Metal feature，会自动使用 GPU
        let path_str = model_path.to_str().context("模型路径包含无效字符")?;
        log::info!("调用 whisper.cpp 加载模型，路径: {}", path_str);

        let ctx = WhisperContext::new_with_params(path_str, params)
            .context("加载 Whisper 模型失败，请检查模型文件是否完整")?;

        self.ctx = Some(ctx);
        self.current_model = Some(
            model_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        );

        log::info!("Whisper 模型加载成功");
        Ok(())
    }

    /// 执行语音识别
    /// audio_data: 16kHz 单声道 f32 PCM 数据
    /// language: 语言代码 ("zh", "en", "auto" 等)
    /// 返回识别文本
    pub fn transcribe(&self, audio_data: &[f32], language: &str) -> Result<String> {
        let ctx = self.ctx.as_ref()
            .context("Whisper 模型未加载，请先加载模型")?;

        // ── 音频数据验证 ──
        let audio_duration_s = audio_data.len() as f32 / 16000.0;
        let rms = audio_rms(audio_data);
        log::info!(
            "音频验证: {} 样本, {:.1}秒, RMS音量={:.6}",
            audio_data.len(), audio_duration_s, rms
        );

        if audio_data.is_empty() {
            anyhow::bail!("音频数据为空");
        }

        // 检测静音：RMS < 0.0005 基本上是无声的
        if rms < 0.0005 {
            anyhow::bail!(
                "录音音量过低 (RMS={:.6})，可能麦克风未正确工作或环境完全静音。\n\
                 请检查: 1. 麦克风是否被静音 2. 系统音频设置中输入设备是否正确",
                rms
            );
        }

        // ── 创建识别参数 ──
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // 设置识别语言
        // 注意：auto 模式需要额外的语言检测步骤，在 CPU 上会更慢
        if language == "auto" || language.is_empty() {
            params.set_language(None);
            log::info!("语言: 自动检测（注意: 会增加 CPU 推理时间）");
        } else {
            params.set_language(Some(language));
            log::info!("语言: {}", language);
        }

        // 性能优化参数
        let n_threads = recommended_threads();
        params.set_n_threads(n_threads);

        // 禁用不必要的输出
        params.set_print_progress(false);
        params.set_print_special(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // 翻译模式：false 表示转录（保持原语言），true 表示翻译成英文
        params.set_translate(false);

        // 短音频优化：5 秒以下使用单段模式，减少开销
        if audio_duration_s < 5.0 {
            params.set_single_segment(true);
            log::info!("短音频模式: 启用 single_segment");
        }

        log::info!(
            "开始 Whisper 推理: 线程={}, 音频={:.1}秒",
            n_threads, audio_duration_s
        );
        let start_time = std::time::Instant::now();

        // ── 创建识别状态并执行识别 ──
        let mut state = ctx.create_state()
            .context("创建 Whisper 状态失败")?;

        log::info!("Whisper state 创建成功，开始 full() 推理...");

        // 执行完整推理（这是最耗时的步骤）
        // Windows CPU 模式下可能非常慢，外层有超时保护
        state.full(params, audio_data)
            .context("Whisper 识别失败")?;

        let elapsed = start_time.elapsed();
        log::info!("Whisper 推理完成，耗时: {:.1}秒", elapsed.as_secs_f64());

        // ── 提取识别结果 ──
        let n_segments = state.full_n_segments()
            .context("获取段落数失败")?;

        log::info!("识别结果: {} 个段落", n_segments);

        let mut result = String::new();
        for i in 0..n_segments {
            let segment_text = state.full_get_segment_text(i)
                .context(format!("获取第 {} 段文本失败", i))?;
            result.push_str(&segment_text);
        }

        // 清理文本：去除首尾空格
        let result = result.trim().to_string();

        log::info!(
            "识别完成: \"{}\" (耗时 {:.1}秒, 实时率 {:.1}x)",
            &result[..result.len().min(50)],
            elapsed.as_secs_f64(),
            elapsed.as_secs_f64() / audio_duration_s as f64,
        );
        Ok(result)
    }

    /// 检查模型是否已加载
    pub fn is_loaded(&self) -> bool {
        self.ctx.is_some()
    }

    /// 获取当前加载的模型名称
    pub fn current_model_name(&self) -> Option<&str> {
        self.current_model.as_deref()
    }

    /// 卸载模型（释放内存）
    pub fn unload(&mut self) {
        self.ctx = None;
        self.current_model = None;
        log::info!("Whisper 模型已卸载");
    }
}

// 让 WhisperEngine 可以在线程间传递
// WhisperContext 在 whisper-rs 中已标记为 Send
unsafe impl Send for WhisperEngine {}
