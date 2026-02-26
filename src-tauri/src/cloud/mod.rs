// cloud/mod.rs - 云端 ASR API 调用
//
// 支持两种协议：
// 1. OpenAI 兼容（multipart/form-data）：OpenAI / 火山引擎 / 讯飞 / 自定义
// 2. 阿里云 NLS RESTful API（裸字节 POST）：阿里云一句话识别

use anyhow::{Context, Result};
use reqwest::multipart;
use serde::Deserialize;
use std::time::Duration;
use crate::state::CloudProvider;

/// 云端识别入参
pub struct CloudTranscribeParams {
    /// 16 kHz 单声道 f32 PCM 数据
    pub audio_samples: Vec<f32>,
    /// "zh" / "en" / "auto"
    pub language: String,
    pub provider: CloudProvider,
    /// OpenAI 兼容：API 的 Base URL（如 https://api.openai.com/v1）
    /// 阿里云 NLS：AppKey（来自控制台项目页）
    pub base_url: String,
    /// OpenAI 兼容：Bearer Token（sk-...）
    /// 阿里云 NLS：X-NLS-Token（来自控制台总览页）
    pub api_key: String,
}

// ===== OpenAI 兼容响应 =====
#[derive(Debug, Deserialize)]
struct OpenAITranscriptionResponse {
    text: String,
}

// ===== 阿里云 NLS 响应 =====
#[derive(Debug, Deserialize)]
struct NlsResponse {
    status: u64,
    result: Option<String>,
    message: Option<String>,
}

// ===== WAV 编码 =====

/// 把 f32 PCM 编码为 WAV 字节（16-bit PCM，单声道，16 kHz）
///
/// WAV 是最通用的格式，OpenAI / 阿里云 NLS 都支持。
/// 文件结构：44 字节 RIFF 头 + PCM 数据
pub fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Vec<u8> {
    // f32（-1.0~1.0）→ i16（-32768~32767）
    let i16_samples: Vec<i16> = samples
        .iter()
        .map(|&s| (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();

    let data_size   = (i16_samples.len() * 2) as u32;
    let byte_rate   = sample_rate * channels as u32 * 2;
    let block_align = channels * 2;

    let mut wav = Vec::with_capacity(44 + data_size as usize);

    // RIFF 头
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36u32 + data_size).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt 子块
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());       // PCM = 1
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());      // 16-bit

    // data 子块
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());

    for s in i16_samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }

    wav
}

// ===== 主入口 =====

/// 执行云端语音识别，根据 provider 分发到对应实现
pub async fn transcribe_cloud(params: CloudTranscribeParams) -> Result<String> {
    match &params.provider {
        // 阿里云走专属 NLS RESTful 接口（裸 WAV POST）
        CloudProvider::Aliyun => transcribe_aliyun_nls(&params).await,
        // 其余服务商走 OpenAI 兼容接口（multipart/form-data）
        _ => transcribe_openai_compatible(params).await,
    }
}

// ===== OpenAI 兼容实现 =====

/// POST /audio/transcriptions（multipart/form-data）
/// 适用于 OpenAI / 火山引擎 / 讯飞 / 自定义
async fn transcribe_openai_compatible(params: CloudTranscribeParams) -> Result<String> {
    let url = format!(
        "{}/audio/transcriptions",
        params.base_url.trim_end_matches('/')
    );
    log::info!("OpenAI 兼容 ASR 请求: {}", url);

    // 编码为 WAV
    let wav_bytes = encode_wav(&params.audio_samples, 16000, 1);
    log::info!("WAV 大小: {} 字节 ({:.1} KB)", wav_bytes.len(), wav_bytes.len() as f64 / 1024.0);

    // 构建 multipart/form-data
    let file_part = multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .context("设置 MIME 类型失败")?;

    let mut form = multipart::Form::new()
        .part("file", file_part)
        .text("model", model_name_for_provider(&params.provider));

    if params.language != "auto" && !params.language.is_empty() {
        form = form.text("language", params.language.clone());
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", params.api_key))
        .multipart(form)
        .send()
        .await
        .context("HTTP 请求失败，请检查网络连接和 API 配置")?;

    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("API 错误 {}: {}", status.as_u16(), body);
    }

    let result: OpenAITranscriptionResponse = resp
        .json()
        .await
        .context("解析 API 响应失败")?;

    let text = result.text.trim().to_string();
    log::info!("识别完成，结果: \"{}\"", &text[..text.len().min(60)]);
    Ok(text)
}

// ===== 阿里云 NLS 实现 =====

/// 阿里云 NLS 一句话识别 RESTful API
///
/// 请求格式（来自官方文档）：
/// ```
/// POST https://nls-gateway-cn-shanghai.aliyuncs.com/stream/v1/asr?appkey={AppKey}
/// X-NLS-Token: {Token}
/// Content-Type: application/octet-stream
/// Body: 裸 WAV 字节
/// ```
///
/// 响应格式：
/// ```json
/// {"task_id":"...","result":"北京的天气","status":20000000,"message":"SUCCESS"}
/// ```
///
/// 字段约定：
/// - `params.base_url` → AppKey（来自控制台项目页）
/// - `params.api_key`  → Token（来自控制台总览页，有效期 24 小时）
async fn transcribe_aliyun_nls(params: &CloudTranscribeParams) -> Result<String> {
    let appkey = params.base_url.trim();
    let token  = params.api_key.trim();

    if appkey.is_empty() {
        anyhow::bail!("阿里云 NLS：请在 AppKey 字段填写控制台的 AppKey");
    }
    if token.is_empty() {
        anyhow::bail!("阿里云 NLS：请在 Token 字段填写控制台的 Token");
    }

    // 编码音频为 WAV（16-bit PCM，单声道，16 kHz，满足阿里云 NLS 要求）
    let wav_bytes = encode_wav(&params.audio_samples, 16000, 1);
    log::info!("阿里云 NLS 请求，AppKey={}, WAV={} 字节", appkey, wav_bytes.len());

    let url = format!(
        "https://nls-gateway-cn-shanghai.aliyuncs.com/stream/v1/asr?appkey={}",
        appkey
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("创建 HTTP 客户端失败")?;

    let resp = client
        .post(&url)
        .header("X-NLS-Token", token)
        .header("Content-Type", "application/octet-stream")
        .body(wav_bytes)
        .send()
        .await
        .context("阿里云 NLS 请求失败，请检查网络和 AppKey/Token")?;

    let nls: NlsResponse = resp
        .json()
        .await
        .context("解析阿里云 NLS 响应失败")?;

    if nls.status == 20000000 {
        let text = nls.result.unwrap_or_default();
        log::info!("阿里云 NLS 识别完成: \"{}\"", &text[..text.len().min(60)]);
        Ok(text)
    } else {
        let msg = nls.message.unwrap_or_else(|| "未知错误".to_string());
        anyhow::bail!("阿里云 NLS 识别失败（状态 {}）: {}", nls.status, msg)
    }
}

/// 测试阿里云 NLS 连通性
///
/// 发送空 body 请求，通过错误码判断鉴权是否通过：
/// - 40000000/40270002（空音频错误）→ 鉴权通过，连接正常
/// - 40000001 → Token 无效
/// - 40020105 → AppKey 不存在
pub async fn test_aliyun_nls(appkey: &str, token: &str) -> Result<String, String> {
    if appkey.trim().is_empty() { return Err("请填写 AppKey".to_string()); }
    if token.trim().is_empty()  { return Err("请填写 Token".to_string()); }

    let url = format!(
        "https://nls-gateway-cn-shanghai.aliyuncs.com/stream/v1/asr?appkey={}",
        appkey.trim()
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建客户端失败: {}", e))?;

    // 空 body POST → 如果 Token/AppKey 有效，服务器返回"空音频"错误而非 401
    let resp = client
        .post(&url)
        .header("X-NLS-Token", token.trim())
        .header("Content-Type", "application/octet-stream")
        .body(vec![])
        .send()
        .await
        .map_err(|e| {
            if e.to_string().contains("dns") || e.to_string().contains("resolve") {
                "域名解析失败，请检查网络".to_string()
            } else {
                format!("连接失败: {}", e)
            }
        })?;

    #[derive(Deserialize)]
    struct NlsResp { status: u64, message: Option<String> }

    let nls: NlsResp = resp.json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    match nls.status {
        // 空音频或无有效语音 → 鉴权通过，服务正常
        40000000 | 40270002 => Ok("连接成功（AppKey 和 Token 有效）".to_string()),
        40000001            => Err("Token 无效或已过期，请重新获取".to_string()),
        40020105            => Err("AppKey 不存在，请检查控制台".to_string()),
        40020106            => Err("AppKey 与 Token 账号不匹配".to_string()),
        40000010            => Err("试用期已结束，请在控制台升级商用版".to_string()),
        code                => Ok(format!("服务可达（状态码 {}）", code)),
    }
}

/// 各 OpenAI 兼容服务商对应的 model 参数
fn model_name_for_provider(provider: &CloudProvider) -> String {
    match provider {
        CloudProvider::OpenAI    => "whisper-1".to_string(),
        CloudProvider::VolcEngine => "Doubao-asr".to_string(),
        CloudProvider::Aliyun   => "paraformer-realtime-v2".to_string(), // 备用（NLS 不用 model）
        CloudProvider::Xunfei   => "iflytekws".to_string(),
        CloudProvider::Custom   => "whisper-1".to_string(),
    }
}
