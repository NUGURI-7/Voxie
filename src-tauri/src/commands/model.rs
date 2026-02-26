// commands/model.rs - 模型下载和管理命令

use tauri::{State, Emitter};
use serde::Serialize;
use crate::state::{AppState, ModelStatus};
use crate::whisper::{WhisperModel, get_model_path, is_model_downloaded};

/// 模型信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,           // 模型名称（tiny/base/small/medium/large-v3）
    pub display_name: String,   // 显示名称
    pub is_downloaded: bool,    // 是否已下载
    pub file_size_mb: f64,      // 文件大小（MB）
}

/// 获取模型状态
#[tauri::command]
pub async fn get_model_status(
    state: State<'_, AppState>,
) -> Result<ModelStatusResponse, String> {
    let inner = state.inner.lock()
        .map_err(|e| format!("获取状态锁失败: {}", e))?;

    Ok(ModelStatusResponse {
        status: inner.model_status.clone(),
        download_progress: inner.download_progress,
        current_model: inner.settings.local_model.clone(),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStatusResponse {
    pub status: ModelStatus,
    pub download_progress: f64,
    pub current_model: String,
}

/// 列出所有模型及其下载状态
#[tauri::command]
pub async fn list_models() -> Result<Vec<ModelInfo>, String> {
    let models = vec![
        WhisperModel::Tiny,
        WhisperModel::Base,
        WhisperModel::Small,
        WhisperModel::Medium,
        WhisperModel::LargeV3,
    ];

    let mut result = Vec::new();
    for model in models {
        let is_downloaded = is_model_downloaded(&model);
        let file_size_mb = if is_downloaded {
            // 读取实际文件大小
            get_model_path(&model)
                .ok()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len() as f64 / 1024.0 / 1024.0)
                .unwrap_or(0.0)
        } else {
            // 预估大小
            match model {
                WhisperModel::Tiny => 39.0,
                WhisperModel::Base => 74.0,
                WhisperModel::Small => 244.0,
                WhisperModel::Medium => 769.0,
                WhisperModel::LargeV3 => 1550.0,
            }
        };

        let name = match model {
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
            WhisperModel::Medium => "medium",
            WhisperModel::LargeV3 => "large-v3",
        };

        result.push(ModelInfo {
            name: name.to_string(),
            display_name: model.display_name().to_string(),
            is_downloaded,
            file_size_mb,
        });
    }

    Ok(result)
}

/// 下载模型命令
/// 使用 Tauri 的事件系统报告下载进度
#[tauri::command]
pub async fn download_model(
    model_name: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let model = WhisperModel::from_str(&model_name)
        .ok_or_else(|| format!("未知的模型名称: {}", model_name))?;

    // 检查是否已下载
    if is_model_downloaded(&model) {
        return Ok(());
    }

    // 更新状态为"下载中"
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("获取状态锁失败: {}", e))?;
        inner.model_status = ModelStatus::Downloading;
        inner.download_progress = 0.0;
    }

    let download_url = model.download_url();
    let model_path = get_model_path(&model)
        .map_err(|e| format!("获取模型路径失败: {}", e))?;

    log::info!("开始下载模型: {} -> {:?}", download_url, model_path);

    // 发送进度事件
    let _ = app.emit("model-download-progress", DownloadProgressEvent {
        model_name: model_name.clone(),
        progress: 0.0,
        status: "downloading".to_string(),
    });

    // 执行下载
    // 注意：这里用 reqwest 的流式下载来跟踪进度
    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("下载请求失败: {}", e))?;

    let total_size = response.content_length().unwrap_or(0);

    // 流式写入文件
    let mut file = std::fs::File::create(&model_path)
        .map_err(|e| format!("创建文件失败: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    use std::io::Write;
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("下载中断: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| format!("写入文件失败: {}", e))?;

        downloaded += chunk.len() as u64;

        // 计算并报告进度
        let progress = if total_size > 0 {
            downloaded as f64 / total_size as f64
        } else {
            0.0
        };

        // 每 5% 更新一次进度（避免过于频繁的事件）
        {
            let mut inner = state.inner.lock()
                .map_err(|e| format!("获取状态锁失败: {}", e))?;
            let old_progress = inner.download_progress;
            if progress - old_progress > 0.05 || progress >= 1.0 {
                inner.download_progress = progress;
                let _ = app.emit("model-download-progress", DownloadProgressEvent {
                    model_name: model_name.clone(),
                    progress,
                    status: "downloading".to_string(),
                });
            }
        }
    }

    // 下载完成，更新状态
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("获取状态锁失败: {}", e))?;
        inner.model_status = ModelStatus::Downloaded;
        inner.download_progress = 1.0;
    }

    let _ = app.emit("model-download-progress", DownloadProgressEvent {
        model_name: model_name.clone(),
        progress: 1.0,
        status: "completed".to_string(),
    });

    log::info!("模型下载完成: {}", model_name);
    Ok(())
}

/// 下载进度事件数据
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    pub model_name: String,
    pub progress: f64,   // 0.0 - 1.0
    pub status: String,  // "downloading" / "completed" / "error"
}

/// 手动将指定模型加载到内存
/// 让用户可以提前加载，避免第一次识别时因加载模型产生无反馈的长时等待
#[tauri::command]
pub async fn load_whisper_model(
    model_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let model = crate::whisper::WhisperModel::from_str(&model_name)
        .ok_or_else(|| format!("未知模型: {}", model_name))?;

    if !crate::whisper::is_model_downloaded(&model) {
        return Err(format!("模型 {} 尚未下载，请先下载", model.display_name()));
    }

    let model_path = crate::whisper::get_model_path(&model)
        .map_err(|e| format!("获取模型路径失败: {}", e))?;

    // 已经加载了同一个模型则跳过
    {
        let eng = state.whisper.lock()
            .map_err(|e| format!("引擎锁失败: {}", e))?;
        if eng.current_model_name() == Some(model.filename()) {
            log::info!("模型 {} 已在内存中，跳过重复加载", model.display_name());
            return Ok(());
        }
    }

    // 设置状态为加载中
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        inner.model_status = ModelStatus::Loading;
    }

    log::info!("手动加载 Whisper 模型: {}", model.display_name());

    // 模型加载是耗时的 blocking 操作，放入专用线程
    let whisper_arc = state.whisper.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut eng = whisper_arc.lock()
            .map_err(|e| format!("引擎锁失败: {}", e))?;
        eng.load_model(&model_path)
            .map_err(|e| format!("加载模型失败: {}", e))
    })
    .await
    .map_err(|e| format!("加载线程崩溃: {}", e))
    .and_then(|r| r)?;

    // 加载完成
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        inner.model_status = ModelStatus::Ready;
    }

    log::info!("模型 {} 手动加载完成", model.display_name());
    Ok(())
}

/// 卸载模型（从内存中释放，保留磁盘文件）
#[tauri::command]
pub async fn unload_whisper_model(
    state: State<'_, AppState>,
) -> Result<(), String> {
    // 卸载引擎（drop WhisperContext，释放 RAM / Metal buffer）
    {
        let mut eng = state.whisper.lock()
            .map_err(|e| format!("引擎锁失败: {}", e))?;
        if !eng.is_loaded() {
            return Ok(()); // 本来就没加载，直接返回
        }
        eng.unload();
    }

    // 更新全局状态：已下载但未加载
    {
        let mut inner = state.inner.lock()
            .map_err(|e| format!("状态锁失败: {}", e))?;
        inner.model_status = ModelStatus::Downloaded;
    }

    log::info!("Whisper 模型已从内存卸载");
    Ok(())
}

/// 删除模型文件
#[tauri::command]
pub async fn delete_model(model_name: String) -> Result<(), String> {
    let model = WhisperModel::from_str(&model_name)
        .ok_or_else(|| format!("未知的模型名称: {}", model_name))?;

    let path = get_model_path(&model)
        .map_err(|e| format!("获取模型路径失败: {}", e))?;

    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("删除模型文件失败: {}", e))?;
        log::info!("已删除模型: {:?}", path);
    }

    Ok(())
}
