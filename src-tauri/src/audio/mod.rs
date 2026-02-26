// audio/mod.rs - 音频录制模块
// 使用 cpal 跨平台录制麦克风音频
//
// 关键设计：
// - 录音时使用设备的原生采样率/声道数（macOS 通常是 44100Hz 双声道）
// - stop() 时将数据重采样到 16000Hz 单声道（Whisper 要求）
// - 这样避免了请求设备不支持的配置而导致 build_input_stream 失败

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// 录音器结构体
/// 封装了 cpal 的音频流，负责从麦克风采集 PCM 数据
pub struct AudioRecorder {
    // cpal 的音频流（启动后开始采集数据）
    stream: Option<cpal::Stream>,
    // 原始录音数据缓冲区（原生采样率、原生声道数）
    buffer: Arc<Mutex<Vec<f32>>>,
    /// 设备原生采样率（Hz），stop() 时用于重采样
    native_sample_rate: u32,
    /// 设备原生声道数，stop() 时用于混音到单声道
    native_channels: usize,
}

impl AudioRecorder {
    /// 创建新的录音器实例
    pub fn new() -> Self {
        AudioRecorder {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            native_sample_rate: 44100, // 保守默认值，start() 会覆盖
            native_channels: 1,
        }
    }

    /// 开始录音
    ///
    /// 使用设备的原生配置（采样率、声道数），不强制要求 16kHz，
    /// 避免设备不支持导致 build_input_stream 失败。
    pub fn start(&mut self) -> Result<()> {
        // 获取默认音频主机（macOS 上是 CoreAudio）
        let host = cpal::default_host();
        log::info!("使用音频主机: {:?}", host.id());

        // 获取默认输入设备（麦克风）
        let device = host
            .default_input_device()
            .context("未找到默认输入设备（麦克风）\n请检查：1.是否授权麦克风权限  2.是否插入麦克风")?;

        let device_name = device.name().unwrap_or_else(|_| "未知设备".to_string());
        log::info!("使用麦克风: {}", device_name);

        // 获取设备支持的默认配置（macOS 通常是 44100Hz / 48000Hz 双声道 f32）
        let supported_config = device
            .default_input_config()
            .context("无法获取设备默认输入配置")?;

        let native_sample_rate = supported_config.sample_rate().0;
        let native_channels   = supported_config.channels() as usize;
        log::info!(
            "设备原生配置: {}Hz, {}ch, {:?}",
            native_sample_rate, native_channels, supported_config.sample_format()
        );

        // 将 SupportedStreamConfig → StreamConfig（保留原生参数）
        let stream_config: cpal::StreamConfig = supported_config.into();

        // 清空缓冲区，准备新的录音
        {
            let mut buf = self.buffer.lock().unwrap();
            buf.clear();
        }

        // 克隆缓冲区引用，供音频回调闭包使用
        let buffer_clone = Arc::clone(&self.buffer);

        // 构建输入流（cpal 负责从设备原生格式转换为 f32）
        let stream = device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                    let mut buf = buffer_clone.lock().unwrap();
                    buf.extend_from_slice(data);
                },
                |err| {
                    log::error!("录音回调错误: {}", err);
                },
                None,
            )
            .context("无法创建音频输入流")?;

        // 启动流
        stream.play().context("无法启动音频流")?;

        // 保存流引用和设备参数
        self.stream              = Some(stream);
        self.native_sample_rate  = native_sample_rate;
        self.native_channels     = native_channels;

        log::info!(
            "录音已开始（{}Hz {}ch → 停止后重采样到 16kHz 单声道）",
            native_sample_rate, native_channels
        );
        Ok(())
    }

    /// 停止录音，返回已重采样到 16000Hz 单声道的 PCM 数据
    pub fn stop(&mut self) -> Vec<f32> {
        // 停止流（drop 触发 cpal 停止采集）
        if let Some(stream) = self.stream.take() {
            drop(stream);
            log::info!("录音流已停止");
        }

        // 取出原始缓冲区数据
        let raw_data = {
            let mut buf = self.buffer.lock().unwrap();
            let data = buf.clone();
            buf.clear();
            data
        };

        log::info!(
            "原始数据: {} 样本（{}Hz {}ch）",
            raw_data.len(), self.native_sample_rate, self.native_channels
        );

        // 重采样 + 混音 → 16kHz 单声道
        const TARGET_RATE: u32 = 16000;
        let resampled = resample_to_mono(
            &raw_data,
            self.native_sample_rate,
            self.native_channels,
            TARGET_RATE,
        );

        let duration_ms = (resampled.len() as f64 / TARGET_RATE as f64 * 1000.0) as u64;

        // 计算音频统计信息（帮助诊断 Windows 上录音问题）
        let rms = crate::whisper::audio_rms(&resampled);
        let max_val = resampled.iter().fold(0.0f32, |m, &s| m.max(s.abs()));
        log::info!(
            "重采样完成: {} 样本, {}ms, RMS={:.6}, 峰值={:.4}",
            resampled.len(), duration_ms, rms, max_val
        );

        if rms < 0.001 {
            log::warn!(
                "录音音量极低 (RMS={:.6})! 可能原因: 麦克风静音/未授权/设备错误",
                rms
            );
        }

        resampled
    }

    /// 检查当前是否正在录音
    pub fn is_recording(&self) -> bool {
        self.stream.is_some()
    }

    /// 获取当前缓冲区中的样本数量（原生采样率）
    pub fn buffer_len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }
}

// cpal::Stream 是线程安全的，显式标记以满足 Tauri 的 Send 要求
unsafe impl Send for AudioRecorder {}

// ===== 重采样工具 =====

/// 多声道原生采样 → 单声道目标采样率（线性插值）
///
/// 两步操作：
/// 1. 按帧混音：多声道取平均 → 单声道
/// 2. 线性插值：从 native_rate 降采样到 target_rate
fn resample_to_mono(
    data: &[f32],
    native_rate: u32,
    native_channels: usize,
    target_rate: u32,
) -> Vec<f32> {
    if data.is_empty() {
        return Vec::new();
    }

    // 第一步：多声道混音为单声道
    let mono: Vec<f32> = if native_channels <= 1 {
        data.to_vec()
    } else {
        data.chunks(native_channels)
            .map(|frame| {
                frame.iter().sum::<f32>() / native_channels as f32
            })
            .collect()
    };

    // 第二步：线性插值重采样
    if native_rate == target_rate {
        return mono;
    }

    let ratio   = native_rate as f64 / target_rate as f64;
    let out_len = ((mono.len() as f64) / ratio).ceil() as usize;
    let mut resampled = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx     = src_pos as usize;
        let frac    = (src_pos - idx as f64) as f32;

        let s0 = mono.get(idx).copied().unwrap_or(0.0);
        let s1 = mono.get(idx + 1).copied().unwrap_or(s0);
        resampled.push(s0 + (s1 - s0) * frac);
    }

    resampled
}

// ===== 工具函数（供其他模块使用）=====

/// 将 PCM f32 数据转换为 i16 格式（WAV 标准格式）
pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
    samples
        .iter()
        .map(|&s| {
            let clamped = s.clamp(-1.0, 1.0);
            (clamped * i16::MAX as f32) as i16
        })
        .collect()
}

/// 计算录音时长（毫秒）
pub fn samples_to_ms(sample_count: usize, sample_rate: u32) -> u64 {
    (sample_count as f64 / sample_rate as f64 * 1000.0) as u64
}
