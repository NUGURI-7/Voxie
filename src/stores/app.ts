// stores/app.ts - Pinia 全局状态管理

import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

// ===== Tauri 环境检测 =====
// 当运行在浏览器（开发预览）而非 Tauri 窗口时，invoke 不可用
// 通过检查 window.__TAURI_INTERNALS__ 判断是否在 Tauri 环境中
const isTauri = typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__

// 懒加载 invoke：只在 Tauri 环境中导入
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    throw new Error(`[Browser Preview] invoke('${cmd}') 不可用，需要在 Tauri 中运行`)
  }
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(cmd, args)
}

// ===== 类型定义（与 Rust 侧对应）=====

export type RecordingStatus = 'idle' | 'recording' | 'processing'
export type ModelStatus = 'notDownloaded' | 'downloading' | 'downloaded' | 'loading' | 'ready' | { error: string }
export type TranscriptionMode = 'local' | 'cloud'
export type CloudProvider = 'openAI' | 'aliyun' | 'custom'
export type AppTheme = 'green' | 'blue' | 'violet' | 'ember' | 'sand' | 'white' | 'gray'

/** 将主题应用到 <html> 的 data-theme 属性 */
export function applyTheme(theme: AppTheme) {
  if (theme === 'green') {
    document.documentElement.removeAttribute('data-theme')
  } else {
    document.documentElement.setAttribute('data-theme', theme)
  }
}

export interface HistoryItem {
  id: string
  text: string
  timestamp: string
  durationMs: number
  mode: TranscriptionMode
  modelName?: string
}

export interface AppSettings {
  mode: TranscriptionMode
  localModel: string
  cloudProvider: CloudProvider
  cloudBaseUrl: string
  cloudApiKey: string
  shortcutKey: string
  language: string
  windowOpacity: number
  autoCopy: boolean
  maxHistory: number
  theme: AppTheme
  /** MyMemory 翻译 Key（可选，留空免费 1000次/天） */
  myMemoryKey: string
}

export type TranslationLang = 'zh-hans' | 'zh-hant' | 'en'

export interface TranslationUsage {
  usedToday: number
  limitToday: number
  hasKey: boolean
}

export interface ModelInfo {
  name: string
  displayName: string
  isDownloaded: boolean
  fileSizeMb: number
}

// ===== 预览模式下的模拟数据 =====
const MOCK_HISTORY: HistoryItem[] = [
  {
    id: '1',
    text: '这是一条示例识别结果，点击可以复制到剪贴板。',
    timestamp: new Date(Date.now() - 60_000).toISOString(),
    durationMs: 3200,
    mode: 'local',
    modelName: 'small',
  },
  {
    id: '2',
    text: 'Hello, this is an English transcription example from Voxie.',
    timestamp: new Date(Date.now() - 300_000).toISOString(),
    durationMs: 5100,
    mode: 'local',
    modelName: 'small',
  },
  {
    id: '3',
    text: '打开设置可以切换本地/云端模式，并下载 Whisper 模型。',
    timestamp: new Date(Date.now() - 900_000).toISOString(),
    durationMs: 4700,
    mode: 'local',
    modelName: 'small',
  },
]

const MOCK_MODELS: ModelInfo[] = [
  { name: 'tiny',     displayName: 'Tiny (~39MB)',    isDownloaded: false, fileSizeMb: 39 },
  { name: 'base',     displayName: 'Base (~74MB)',    isDownloaded: false, fileSizeMb: 74 },
  { name: 'small',    displayName: 'Small (~244MB)',  isDownloaded: true,  fileSizeMb: 244 },
  { name: 'medium',   displayName: 'Medium (~769MB)', isDownloaded: false, fileSizeMb: 769 },
  { name: 'large-v3', displayName: 'Large-v3 (~1.5GB)', isDownloaded: false, fileSizeMb: 1550 },
]

// ===== 主 Store =====

export const useAppStore = defineStore('app', () => {
  // ===== 状态 =====
  const recordingStatus = ref<RecordingStatus>('idle')
  const modelStatus = ref<ModelStatus>('notDownloaded')
  const downloadProgress = ref(0)
  const currentModel = ref('small')
  // 当前已加载到内存的模型名称（区别于 settings.localModel 只是"选中"）
  const loadedModelName = ref<string | null>(null)
  const history = ref<HistoryItem[]>([])
  const settings = ref<AppSettings>({
    mode: 'local',
    localModel: 'small',
    cloudProvider: 'openAI',
    cloudBaseUrl: 'https://api.openai.com/v1',
    cloudApiKey: '',
    shortcutKey: 'Alt',
    language: 'auto',
    windowOpacity: 0.85,
    autoCopy: true,
    maxHistory: 100,
    theme: 'green',
    myMemoryKey: '',
  })
  const isCollapsed = ref(false)
  const toast = ref<{ message: string; type: 'success' | 'error' | 'info' } | null>(null)
  /** 从转录页点击"翻译"后，待填入翻译框的文本 */
  const pendingTranslationText = ref('')
  const models = ref<ModelInfo[]>([])
  const translationUsage = ref<TranslationUsage>({ usedToday: 0, limitToday: 1000, hasKey: false })

  // ===== 计算属性 =====
  const isRecording = computed(() => recordingStatus.value === 'recording')
  const isProcessing = computed(() => recordingStatus.value === 'processing')
  const isModelReady = computed(() => modelStatus.value === 'ready')
  const latestItem = computed(() => history.value[0] ?? null)

  // ===== 方法 =====

  async function loadSettings() {
    if (!isTauri) {
      // 浏览器预览：使用默认设置，不报错
      applyTheme(settings.value.theme)
      return
    }
    try {
      const s = await tauriInvoke<AppSettings>('get_settings')
      // 兼容旧版本持久化数据（无新字段）
      settings.value = {
        ...s,
        theme: s.theme || 'green',
      }
      applyTheme(settings.value.theme)
    } catch (e) {
      console.error('加载设置失败:', e)
    }
  }

  async function saveSettings() {
    if (!isTauri) {
      showToast('设置已保存（预览模式）', 'success')
      return
    }
    try {
      await tauriInvoke('save_settings', { settings: settings.value })
      showToast('设置已保存', 'success')
    } catch (e) {
      showToast('保存设置失败', 'error')
    }
  }

  async function loadHistory() {
    if (!isTauri) {
      // 浏览器预览：加载模拟数据
      history.value = MOCK_HISTORY
      return
    }
    try {
      const items = await tauriInvoke<HistoryItem[]>('get_history')
      history.value = items
    } catch (e) {
      console.error('加载历史记录失败:', e)
    }
  }

  async function clearHistory() {
    if (!isTauri) {
      history.value = []
      showToast('历史记录已清空', 'success')
      return
    }
    try {
      await tauriInvoke('clear_history')
      history.value = []
      showToast('历史记录已清空', 'success')
    } catch (e) {
      console.error('清空历史记录失败:', e)
    }
  }

  async function deleteHistoryItem(id: string) {
    if (!isTauri) {
      history.value = history.value.filter(item => item.id !== id)
      return
    }
    try {
      await tauriInvoke('delete_history_item', { id })
      history.value = history.value.filter(item => item.id !== id)
    } catch (e) {
      console.error('删除历史记录失败:', e)
    }
  }

  async function startRecording() {
    if (!isTauri) {
      recordingStatus.value = 'recording'
      return
    }
    try {
      await tauriInvoke('start_recording')
      recordingStatus.value = 'recording'
    } catch (e) {
      showToast(`录音失败: ${e}`, 'error')
    }
  }

  async function stopRecording() {
    if (!isTauri) {
      recordingStatus.value = 'processing'
      setTimeout(() => {
        recordingStatus.value = 'idle'
        const mockItem: HistoryItem = {
          id: Date.now().toString(),
          text: '（预览模式）这是一条模拟识别结果。',
          timestamp: new Date().toISOString(),
          durationMs: 2000,
          mode: 'local',
        }
        history.value.unshift(mockItem)
        showToast('已复制到剪贴板', 'success')
      }, 1500)
      return
    }
    try {
      await tauriInvoke('stop_recording')
      recordingStatus.value = 'processing'

      // 本地模式下提示用户 CPU 推理可能较慢
      if (settings.value.mode === 'local') {
        showToast('正在识别中，请稍候...', 'info')
      }

      const result = await tauriInvoke<{ text: string; durationMs: number; itemId: string }>('transcribe_audio')
      if (settings.value.autoCopy && result.text) {
        await copyToClipboard(result.text)
      }
      recordingStatus.value = 'idle'
      await loadHistory()
    } catch (e) {
      recordingStatus.value = 'idle'
      const errMsg = String(e)
      // 对常见错误提供更友好的提示
      if (errMsg.includes('超时')) {
        showToast('识别超时，建议使用更小的模型或切换云端模式', 'error')
      } else if (errMsg.includes('音量过低')) {
        showToast('录音音量过低，请检查麦克风设置', 'error')
      } else {
        showToast(`识别失败: ${errMsg}`, 'error')
      }
    }
  }

  async function copyToClipboard(text: string) {
    if (!isTauri) {
      // 浏览器预览：使用 navigator.clipboard
      try {
        await navigator.clipboard.writeText(text)
        showToast('已复制到剪贴板', 'success')
      } catch {
        showToast('复制失败', 'error')
      }
      return
    }
    try {
      await tauriInvoke('copy_to_clipboard', { text })
      showToast('已复制到剪贴板', 'success')
    } catch (e) {
      showToast('复制失败', 'error')
    }
  }

  function showToast(message: string, type: 'success' | 'error' | 'info' = 'info') {
    toast.value = { message, type }
    setTimeout(() => { toast.value = null }, 3000)
  }

  async function loadModels() {
    if (!isTauri) {
      models.value = MOCK_MODELS
      return
    }
    try {
      models.value = await tauriInvoke<ModelInfo[]>('list_models')
    } catch (e) {
      console.error('加载模型列表失败:', e)
    }
  }

  /** 翻译文字：简↔繁 本地完成，其他方向调 MyMemory API */
  async function translateText(
    text: string,
    from: TranslationLang,
    to: TranslationLang,
  ): Promise<string> {
    if (!isTauri) {
      // 预览模式：模拟延迟并递增用量计数
      await new Promise(r => setTimeout(r, 600))
      translationUsage.value = {
        ...translationUsage.value,
        usedToday: translationUsage.value.usedToday + 1,
      }
      return `[预览] ${text}`
    }
    const result = await tauriInvoke<string>('translate_text', { text, from, to })
    return result
  }

  /** 查询今日翻译用量 */
  async function getTranslationUsage(): Promise<void> {
    if (!isTauri) return
    try {
      translationUsage.value = await tauriInvoke<TranslationUsage>('get_translation_usage')
    } catch (e) {
      console.error('获取翻译用量失败:', e)
    }
  }

  /** 卸载模型（释放 RAM / Metal buffer），保留磁盘文件 */
  async function unloadWhisperModel(): Promise<void> {
    if (!isTauri) {
      modelStatus.value = 'downloaded'
      loadedModelName.value = null
      showToast('模型已卸载（预览模式）', 'success')
      return
    }
    try {
      await tauriInvoke('unload_whisper_model')
      modelStatus.value = 'downloaded'
      loadedModelName.value = null
      showToast('模型已从内存卸载', 'success')
    } catch (e) {
      showToast(`卸载失败: ${e}`, 'error')
    }
  }

  /** 手动加载指定模型到内存，给用户明确的加载状态反馈 */
  async function loadWhisperModel(modelName: string): Promise<void> {
    if (!isTauri) {
      modelStatus.value = 'loading'
      await new Promise(r => setTimeout(r, 1200))  // 模拟加载
      modelStatus.value = 'ready'
      loadedModelName.value = modelName
      showToast('模型已加载（预览模式）', 'success')
      return
    }
    modelStatus.value = 'loading'
    try {
      await tauriInvoke('load_whisper_model', { modelName })
      modelStatus.value = 'ready'
      loadedModelName.value = modelName
      showToast('模型已就绪', 'success')
    } catch (e) {
      modelStatus.value = { error: String(e) }
      showToast(`模型加载失败: ${e}`, 'error')
      throw e
    }
  }

  async function downloadModel(modelName: string) {
    if (!isTauri) {
      showToast('下载功能需要在 Tauri 中运行', 'info')
      return
    }
    try {
      modelStatus.value = 'downloading'
      downloadProgress.value = 0
      await tauriInvoke('download_model', { modelName })
      modelStatus.value = 'downloaded'
      await loadModels()
      showToast('模型下载完成', 'success')
    } catch (e) {
      modelStatus.value = 'notDownloaded'
      showToast(`模型下载失败: ${e}`, 'error')
    }
  }

  // ===== 测试云端连接 =====
  async function testCloudConnection(
    baseUrl: string,
    apiKey: string,
    provider: string = '',
  ): Promise<{ ok: boolean; message: string }> {
    if (!isTauri) {
      await new Promise(r => setTimeout(r, 800))
      return { ok: true, message: '连接成功（预览模式）' }
    }
    try {
      const msg = await tauriInvoke<string>('test_cloud_connection', { baseUrl, apiKey, provider })
      return { ok: true, message: msg }
    } catch (e) {
      return { ok: false, message: String(e) }
    }
  }

  return {
    recordingStatus, modelStatus, downloadProgress, currentModel, loadedModelName,
    history, settings, isCollapsed, toast, models,
    isRecording, isProcessing, isModelReady, latestItem,
    loadSettings, saveSettings, loadHistory, clearHistory, deleteHistoryItem,
    startRecording, stopRecording, copyToClipboard, showToast,
    loadModels, loadWhisperModel, unloadWhisperModel, downloadModel, testCloudConnection,
    translationUsage, translateText, getTranslationUsage,
    pendingTranslationText,
    isTauri, applyTheme,
  }
})
