<template>
  <div class="sp h-full flex flex-col">

    <!-- ===== 标题栏 ===== -->
    <div class="sp-bar select-none" data-tauri-drag-region>
      <div class="sp-bar__left" data-tauri-drag-region>
        <!-- 返回按钮 -->
        <button class="sp-back no-drag" @click="goBack" title="返回">
          <ArrowLeft :size="16" :stroke-width="2.5" />
        </button>
        <span class="sp-title" data-tauri-drag-region>设置</span>
      </div>

      <!-- 保存按钮 -->
      <button class="sp-save no-drag" @click="saveSettings">保存</button>
    </div>

    <!-- ===== 设置内容（可滚动）===== -->
    <div class="sp-body">

      <!-- ===== 识别模式 ===== -->
      <div class="sp-section">
        <p class="sp-label">识别模式</p>
        <div class="seg">
          <button
            v-for="m in ['local', 'cloud']" :key="m"
            class="seg__item no-drag"
            :class="{ 'seg__item--on': localSettings.mode === m }"
            @click="localSettings.mode = m as any"
          >
            {{ m === 'local' ? '本地 Whisper' : '云端 API' }}
          </button>
        </div>
      </div>

      <!-- ===== 本地模型 ===== -->
      <div v-if="localSettings.mode === 'local'" class="sp-section">
        <p class="sp-label">本地模型</p>

        <!-- 模型状态卡片 -->
        <div class="model-status-card">
          <div class="flex items-center justify-between">
            <span class="msc-name">模型状态</span>
            <span class="msc-val" :class="modelStatusClass">{{ modelStatusText }}</span>
          </div>
          <!-- 下载进度 -->
          <div v-if="appStore.modelStatus === 'downloading'" class="msc-progress">
            <div class="msc-track">
              <div class="msc-fill"
                :style="{ width: `${Math.round(appStore.downloadProgress * 100)}%` }">
              </div>
            </div>
            <span class="msc-pct">{{ Math.round(appStore.downloadProgress * 100) }}%</span>
          </div>
        </div>

        <!-- 模型列表 -->
        <div class="model-list">
          <button
            v-for="model in availableModels" :key="model.name"
            class="model-item no-drag"
            :class="{
              'model-item--on': localSettings.localModel === model.name,
              'model-item--deleting': deletingModel === model.name
            }"
            @click="localSettings.localModel = model.name"
          >
            <!-- 单选点 -->
            <div class="model-radio"
              :class="{ 'model-radio--on': localSettings.localModel === model.name }">
            </div>

            <!-- 模型名 -->
            <span class="model-name">{{ model.displayName }}</span>

            <!-- 右侧操作 -->
            <div class="model-actions">
              <!-- 未下载：显示预估大小 + 下载按钮 -->
              <template v-if="!model.isDownloaded">
                <span class="model-size">{{ model.fileSizeMb.toFixed(0) }} MB</span>
                <button
                  class="model-btn model-btn--dl no-drag"
                  :disabled="appStore.modelStatus === 'downloading'"
                  @click.stop="downloadModel(model.name)"
                  title="下载"
                >
                  <Download :size="13" :stroke-width="2" />
                </button>
              </template>

              <!-- 已下载：就绪 / 加载中 / 加载按钮 + 删除 -->
              <template v-else>
                <!-- 状态徽章 -->
                <span v-if="appStore.loadedModelName === model.name && appStore.modelStatus === 'ready'"
                  class="model-loaded">
                  <Zap :size="10" :stroke-width="2.5" />就绪
                </span>
                <span v-else-if="appStore.loadedModelName === model.name && appStore.modelStatus === 'loading'"
                  class="model-loading-badge">
                  加载中...
                </span>
                <span v-else class="model-downloaded">已下载</span>

                <!-- 加载按钮（未就绪且当前没有在加载中时显示） -->
                <button
                  v-if="appStore.loadedModelName !== model.name"
                  class="model-btn model-btn--load no-drag"
                  :disabled="appStore.modelStatus === 'loading' || appStore.modelStatus === 'downloading'"
                  @click.stop="loadModelToMemory(model.name)"
                  title="加载到内存"
                >
                  <Loader2 v-if="loadingModel === model.name"
                    :size="13" :stroke-width="2" class="spin-del" />
                  <Play v-else :size="11" :stroke-width="2.5" />
                </button>

                <!-- 卸载按钮（已就绪时显示，释放 RAM 但保留磁盘文件） -->
                <button
                  v-if="appStore.loadedModelName === model.name && appStore.modelStatus === 'ready'"
                  class="model-btn model-btn--unload no-drag"
                  :disabled="unloadingModel"
                  @click.stop="unloadModel"
                  title="从内存卸载（保留文件）"
                >
                  <Loader2 v-if="unloadingModel" :size="13" :stroke-width="2" class="spin-del" />
                  <StopCircle v-else :size="13" :stroke-width="2" />
                </button>

                <!-- 删除按钮 -->
                <button
                  class="model-btn no-drag"
                  :class="deletingModel === model.name ? 'model-btn--deleting' : 'model-btn--del'"
                  :disabled="deletingModel === model.name || appStore.modelStatus === 'loading'"
                  @click.stop="deleteModel(model.name)"
                  :title="deletingModel === model.name ? '删除中...' : '删除'"
                >
                  <Loader2 v-if="deletingModel === model.name"
                    :size="13" :stroke-width="2" class="spin-del" />
                  <TrashIcon v-else :size="13" :stroke-width="2" />
                </button>
              </template>
            </div>
          </button>
        </div>
      </div>

      <!-- ===== 云端 API ===== -->
      <div v-else class="sp-section">
        <p class="sp-label">云端 API</p>
        <div class="field-stack">

          <!-- 服务商 -->
          <div class="field">
            <label class="field__lbl">服务商</label>
            <select
              v-model="localSettings.cloudProvider"
              class="field__select no-drag"
              @change="onProviderChange"
            >
              <option value="openAI">OpenAI</option>
              <option value="aliyun">阿里云 NLS（一句话识别）</option>
              <option value="custom">自定义</option>
            </select>
          </div>

          <!-- Base URL / AppKey（阿里云 NLS 用 AppKey）-->
          <div class="field">
            <label class="field__lbl">
              {{ localSettings.cloudProvider === 'aliyun' ? 'AppKey' : 'Base URL' }}
            </label>
            <input
              v-model="localSettings.cloudBaseUrl"
              :type="localSettings.cloudProvider === 'aliyun' ? 'text' : 'url'"
              class="field__input no-drag"
              :placeholder="localSettings.cloudProvider === 'aliyun'
                ? '控制台 → 项目管理 → AppKey'
                : 'https://api.openai.com/v1'"
            />
            <!-- 阿里云 NLS 说明 -->
            <p v-if="localSettings.cloudProvider === 'aliyun'" class="field__hint">
              在<strong>智能语音交互控制台</strong>创建项目后获取 AppKey。
            </p>
          </div>

          <!-- API Key / Token（阿里云 NLS 用 X-NLS-Token）-->
          <div class="field">
            <label class="field__lbl">
              {{ localSettings.cloudProvider === 'aliyun' ? 'Token (X-NLS-Token)' : 'API Key' }}
            </label>
            <div class="field__pwd-wrap">
              <input
                v-model="localSettings.cloudApiKey"
                :type="showApiKey ? 'text' : 'password'"
                class="field__input no-drag"
                :placeholder="localSettings.cloudProvider === 'aliyun'
                  ? '控制台总览页获取，有效期 24 小时'
                  : 'sk-...'"
              />
              <button class="field__eye no-drag" @click="showApiKey = !showApiKey">
                <Eye v-if="!showApiKey" :size="14" :stroke-width="2" />
                <EyeOff v-else :size="14" :stroke-width="2" />
              </button>
            </div>
          </div>

          <!-- 测试连接 -->
          <div class="field">
            <div class="test-row">
              <button
                class="test-btn no-drag"
                :disabled="testLoading"
                @click="testConnection"
              >
                <Wifi v-if="!testLoading" :size="14" :stroke-width="2" />
                <Loader2 v-else :size="14" :stroke-width="2" class="test-spin" />
                {{ testLoading ? '测试中...' : '测试连接' }}
              </button>

              <!-- 测试结果 -->
              <span v-if="testResult" class="test-result"
                :class="testResult.ok ? 'test-result--ok' : 'test-result--err'">
                <CheckCircle2 v-if="testResult.ok" :size="11" class="test-ico" :stroke-width="2.5" />
                <XCircle v-else :size="11" class="test-ico" :stroke-width="2.5" />
                {{ testResult.message }}
              </span>
            </div>
          </div>

        </div>
      </div>

      <!-- ===== 通用设置 ===== -->
      <div class="sp-section">
        <p class="sp-label">通用</p>
        <div class="field-stack">

          <!-- 语言 -->
          <div class="field">
            <label class="field__lbl">识别语言</label>
            <select v-model="localSettings.language" class="field__select no-drag">
              <option value="auto">自动检测</option>
              <option value="zh">中文</option>
              <option value="en">English</option>
              <option value="ja">日本語</option>
              <option value="ko">한국어</option>
            </select>
          </div>

          <!-- 窗口透明度 -->
          <div class="field">
            <div class="flex items-center justify-between mb-1">
              <label class="field__lbl" style="margin-bottom: 0">窗口透明度</label>
              <span class="opacity-val">{{ Math.round(localSettings.windowOpacity * 100) }}%</span>
            </div>
            <input
              v-model.number="localSettings.windowOpacity"
              type="range"
              min="0.3" max="1.0" step="0.05"
              class="field__range no-drag"
            />
          </div>

          <!-- 自动复制 -->
          <div class="field-row">
            <span class="field__lbl">识别后自动复制</span>
            <label class="toggle-wrap no-drag">
              <input
                v-model="localSettings.autoCopy"
                type="checkbox"
                class="toggle-input"
              />
              <span class="toggle-track">
                <span class="toggle-thumb"></span>
              </span>
            </label>
          </div>

        </div>
      </div>

      <!-- ===== 外观主题 ===== -->
      <div class="sp-section">
        <p class="sp-label">主题色</p>
        <div class="theme-grid">
          <button
            v-for="t in themes" :key="t.id"
            class="theme-swatch no-drag"
            :class="{ 'theme-swatch--on': localSettings.theme === t.id }"
            @click="pickTheme(t.id)"
            :title="t.label"
          >
            <span class="theme-dot" :style="{ background: t.color }">
              <svg v-if="localSettings.theme === t.id"
                viewBox="0 0 10 10" fill="none"
                class="theme-check">
                <path d="M2 5.5L4.2 7.8L8 3" stroke="white" stroke-width="1.6"
                  stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </span>
            <span class="theme-name">{{ t.label }}</span>
          </button>
        </div>
      </div>

      <!-- ===== 翻译设置 ===== -->
      <div class="sp-section">
        <p class="sp-label">翻译</p>
        <div class="field-stack">

          <!-- MyMemory Key -->
          <div class="field">
            <label class="field__lbl">MyMemory Key（可选）</label>
            <div class="field__pwd-wrap">
              <input
                v-model="localSettings.myMemoryKey"
                :type="showMmKey ? 'text' : 'password'"
                class="field__input no-drag"
                placeholder="留空免费使用（1000 次/天）"
                autocomplete="off"
              />
              <button class="field__eye no-drag" @click="showMmKey = !showMmKey">
                <Eye v-if="!showMmKey" :size="14" :stroke-width="2" />
                <EyeOff v-else :size="14" :stroke-width="2" />
              </button>
            </div>
            <p class="field__hint">
              填入 Key 可提升至 10000 次/天。
              免费注册：<span class="field__link">mymemory.translated.net</span>
            </p>
          </div>

          <!-- 今日用量 -->
          <div class="field-row">
            <span class="field__lbl">今日翻译用量</span>
            <span class="quota-tag" :class="quotaTagClass">{{ quotaTagText }}</span>
          </div>

        </div>
      </div>

      <!-- ===== 数据管理 ===== -->
      <div class="sp-section sp-section--last">
        <p class="sp-label">数据</p>
        <button class="danger-btn no-drag" @click="clearHistory">
          <TrashIcon :size="14" :stroke-width="2" />
          清空历史记录
        </button>
      </div>

      <!-- 底部留白 -->
      <div style="height: 16px;"></div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import type { AppSettings, ModelInfo } from '@/stores/app'
import { ArrowLeft, Wifi, Eye, EyeOff, Download, Trash2 as TrashIcon, CheckCircle2, XCircle, Loader2, Play, Zap, StopCircle } from 'lucide-vue-next'
import { applyTheme } from '@/stores/app'
import type { AppTheme } from '@/stores/app'

const router = useRouter()
const appStore = useAppStore()

// 本地设置副本：避免直接修改 store（保存前不生效）
const localSettings = reactive<AppSettings>({ ...appStore.settings })
const showApiKey = ref(false)
const showMmKey  = ref(false)
const availableModels = ref<ModelInfo[]>([])

// 记录进入设置时的原始值，用于放弃更改时还原
const originalOpacity = appStore.settings.windowOpacity

// 透明度滑块：实时预览（无需点保存就能看到效果）
watch(() => localSettings.windowOpacity, (newVal) => {
  appStore.settings.windowOpacity = newVal   // 立刻更新 CSS var
  // 同步通知 Rust（让 set_window_opacity 写日志 / 将来可做额外逻辑）
  if (appStore.isTauri) {
    import('@tauri-apps/api/core').then(({ invoke }) => {
      invoke('set_window_opacity', { opacity: newVal }).catch(() => {})
    })
  }
})

// ===== 测试连接状态 =====
const testLoading = ref(false)
const testResult = ref<{ ok: boolean; message: string } | null>(null)

let unlistenProgress: (() => void) | null = null

// ===== 翻译用量显示 =====
const quotaTagText = computed(() => {
  const u = appStore.translationUsage
  if (u.usedToday === 0 && !u.hasKey) return '尚未使用'
  const remaining = Math.max(0, u.limitToday - u.usedToday)
  return `剩余 ${remaining} / ${u.limitToday}`
})

const quotaTagClass = computed(() => {
  const u = appStore.translationUsage
  const remaining = u.limitToday - u.usedToday
  if (remaining <= 0) return 'quota-tag--empty'
  if (remaining < u.limitToday * 0.1) return 'quota-tag--low'
  return 'quota-tag--ok'
})

onMounted(async () => {
  // 先异步加载模型列表，不阻塞组件渲染
  loadModels()
  // 加载翻译用量
  appStore.getTranslationUsage()

  // 只在 Tauri 环境中监听下载进度
  if (appStore.isTauri) {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenProgress = await listen<{ modelName: string; progress: number; status: string }>(
      'model-download-progress',
      (event) => {
        appStore.downloadProgress = event.payload.progress
        if (event.payload.status === 'completed') {
          loadModels()
        }
      }
    )
  }
})

onUnmounted(() => {
  if (unlistenProgress) unlistenProgress()
})

async function loadModels() {
  await appStore.loadModels()
  availableModels.value = appStore.models
}

// ===== 模型状态显示（基于当前选中的模型，而非全局状态）=====

/** 当前选中模型的 ModelInfo */
const selectedModelInfo = computed(() =>
  availableModels.value.find(m => m.name === localSettings.localModel)
)

const modelStatusClass = computed(() => {
  const model = selectedModelInfo.value
  const s = appStore.modelStatus

  // 正在下载/加载（无论哪个模型，优先展示进行中状态）
  if (s === 'downloading') return 'msc-val--warn'
  if (s === 'loading' && loadingModel.value === model?.name) return 'msc-val--warn'

  if (!model?.isDownloaded) return ''        // 未下载 → 灰色
  if (typeof s === 'object') return 'msc-val--err'

  // 已下载且是当前加载的模型
  if (appStore.loadedModelName === model.name && s === 'ready') return 'msc-val--ok'

  return ''  // 已下载但未加载 → 灰色
})

const modelStatusText = computed(() => {
  const model = selectedModelInfo.value
  const s = appStore.modelStatus

  // 正在下载
  if (s === 'downloading')
    return `下载中 ${Math.round(appStore.downloadProgress * 100)}%`

  // 正在加载当前选中的模型
  if (s === 'loading' && loadingModel.value === model?.name)
    return '加载中...'

  if (!model) return '未知'

  // 未下载
  if (!model.isDownloaded) return '未下载'

  // 已下载 + 是当前加载的模型 + 就绪
  if (appStore.loadedModelName === model.name && s === 'ready') return '就绪'

  // 错误
  if (typeof s === 'object') return `错误: ${(s as any).error}`

  // 已下载但未加载到内存
  return '已下载（未加载）'
})

// ===== 服务商切换：自动填入默认值 =====
function onProviderChange() {
  // 阿里云 NLS 的 cloudBaseUrl 字段存 AppKey（不是 URL），切换时清空让用户自填
  const defaults: Record<string, string> = {
    openAI: 'https://api.openai.com/v1',
    aliyun: '',   // AppKey 字段，用户自填
    custom: '',
  }
  localSettings.cloudBaseUrl = defaults[localSettings.cloudProvider] ?? ''
  localSettings.cloudApiKey  = ''  // 切换服务商时清空 Key/Token
  testResult.value = null          // 清空上次测试结果
}

// ===== 模型操作 =====
async function downloadModel(modelName: string) {
  await appStore.downloadModel(modelName)
  await loadModels()
}

const deletingModel  = ref<string | null>(null)
const loadingModel   = ref<string | null>(null)
const unloadingModel = ref(false)

async function unloadModel() {
  unloadingModel.value = true
  try {
    await appStore.unloadWhisperModel()
  } finally {
    unloadingModel.value = false
  }
}

async function loadModelToMemory(modelName: string) {
  // 同步 localSettings（让加载按钮关联到正确模型）
  localSettings.localModel = modelName
  loadingModel.value = modelName
  try {
    await appStore.loadWhisperModel(modelName)
  } catch {
    // 错误已在 store 里 showToast
  } finally {
    loadingModel.value = null
  }
}

async function deleteModel(modelName: string) {
  if (!appStore.isTauri) {
    availableModels.value = availableModels.value.map(m =>
      m.name === modelName ? { ...m, isDownloaded: false } : m
    )
    appStore.showToast('已删除（预览模式）', 'success')
    return
  }
  deletingModel.value = modelName
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    await invoke('delete_model', { modelName })
    await loadModels()
    appStore.showToast('模型已删除', 'success')
  } catch (e) {
    appStore.showToast(`删除失败: ${e}`, 'error')
  } finally {
    deletingModel.value = null
  }
}

// ===== 测试云端连接 =====
async function testConnection() {
  testLoading.value = true
  testResult.value = null
  testResult.value = await appStore.testCloudConnection(
    localSettings.cloudBaseUrl,
    localSettings.cloudApiKey,
    localSettings.cloudProvider,   // 告诉后端用哪套协议
  )
  testLoading.value = false
}

// ===== 主题 =====
const themes: { id: AppTheme; label: string; color: string }[] = [
  { id: 'green',  label: '森绿', color: '#16A34A' },
  { id: 'blue',   label: '海蓝', color: '#2563EB' },
  { id: 'violet', label: '浅紫', color: '#7C3AED' },
  { id: 'ember',  label: '暖橙', color: '#EA580C' },
  { id: 'sand',   label: '奶油', color: '#78716C' },
  { id: 'white',  label: '纯白', color: '#0A84FF' },
  { id: 'gray',   label: '浅灰', color: '#6366F1' },
]

function pickTheme(id: AppTheme) {
  localSettings.theme = id
  applyTheme(id)   // 即时预览
}

// ===== 保存 & 返回 =====
async function saveSettings() {
  Object.assign(appStore.settings, localSettings)
  await appStore.saveSettings()
  router.back()
}

async function clearHistory() {
  await appStore.clearHistory()
}

function goBack() {
  // 放弃更改时还原主题和透明度预览
  applyTheme(appStore.settings.theme)
  appStore.settings.windowOpacity = originalOpacity   // 还原透明度
  router.back()
}
</script>

<style scoped>
/* ===== 根容器 ===== */
.sp {
  color: #0F172A;
}

/* ===== 标题栏 ===== */
.sp-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 6px;
  height: 40px;
  background: rgba(255, 255, 255, 0.55);
  border-bottom: 1px solid rgba(var(--clr-accent-rgb), 0.12);
  flex-shrink: 0;
}

.sp-bar__left {
  display: flex;
  align-items: center;
  gap: 2px;
}

.sp-back {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #94A3B8;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}

.sp-back:hover {
  color: var(--clr-accent);
  background: rgba(var(--clr-accent-rgb), 0.08);
}

.sp-back svg { width: 14px; height: 14px; }

.sp-title {
  font-size: 13px;
  font-weight: 700;
  color: #0F172A;
}

.sp-save {
  font-size: 12px;
  font-weight: 700;
  color: var(--clr-accent);
  padding: 5px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
  letter-spacing: 0.01em;
}

.sp-save:hover {
  color: var(--clr-accent-dark);
  background: rgba(var(--clr-accent-rgb), 0.10);
}

/* ===== 主内容滚动区 ===== */
.sp-body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--clr-accent-rgb), 0.18) transparent;
}

/* ===== 设置分节 ===== */
.sp-section {
  padding-bottom: 14px;
  margin-bottom: 14px;
  border-bottom: 1px solid rgba(var(--clr-accent-rgb), 0.08);
}

.sp-section--last {
  border-bottom: none;
  margin-bottom: 0;
}

.sp-label {
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #94A3B8;
  margin-bottom: 8px;
}

/* ===== 分段控制器（模式切换）===== */
.seg {
  display: flex;
  background: rgba(var(--clr-accent-rgb), 0.06);
  border-radius: 9px;
  padding: 3px;
  gap: 3px;
  border: 1px solid rgba(var(--clr-accent-rgb), 0.10);
}

.seg__item {
  flex: 1;
  padding: 6px 0;
  font-size: 12px;
  font-weight: 500;
  border-radius: 7px;
  color: #94A3B8;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.seg__item--on {
  background: rgba(var(--clr-accent-rgb), 0.14);
  color: var(--clr-accent);
  font-weight: 600;
}

.seg__item:not(.seg__item--on):hover {
  background: rgba(var(--clr-accent-rgb), 0.07);
  color: #475569;
}

/* ===== 模型状态卡片 ===== */
.model-status-card {
  background: rgba(var(--bg-window-tint), 0.70);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.12);
  border-radius: 9px;
  padding: 8px 10px;
  margin-bottom: 8px;
}

.msc-name {
  font-size: 11px;
  color: #94A3B8;
}

.msc-val {
  font-size: 11px;
  font-weight: 600;
  color: #94A3B8;
}

.msc-val--ok   { color: var(--clr-accent); }
.msc-val--warn { color: #D97706; }
.msc-val--err  { color: #EF4444; }

.msc-progress {
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.msc-track {
  flex: 1;
  height: 3px;
  background: rgba(var(--clr-accent-rgb), 0.12);
  border-radius: 2px;
  overflow: hidden;
}

.msc-fill {
  height: 100%;
  background: var(--clr-accent);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.msc-pct {
  font-size: 10px;
  color: #94A3B8;
  min-width: 28px;
  text-align: right;
}

/* ===== 模型列表 ===== */
.model-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.model-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: 9px;
  border: 1px solid rgba(0, 0, 0, 0.07);
  background: rgba(255, 255, 255, 0.60);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
  width: 100%;
  text-align: left;
}

.model-item--on {
  border-color: rgba(var(--clr-accent-rgb), 0.35);
  background: rgba(var(--bg-window-tint), 0.80);
  box-shadow: 0 0 0 2px rgba(var(--clr-accent-rgb), 0.08);
}

.model-item:not(.model-item--on):hover {
  background: rgba(255, 255, 255, 0.80);
  border-color: rgba(0, 0, 0, 0.10);
}

.model-radio {
  width: 13px;
  height: 13px;
  border-radius: 50%;
  border: 1.5px solid #CBD5E1;
  flex-shrink: 0;
  transition: border-color 0.15s, background 0.15s;
}

.model-radio--on {
  border-color: var(--clr-accent);
  background: var(--clr-accent);
  box-shadow: inset 0 0 0 2px #fff;
}

.model-name {
  font-size: 12px;
  color: #0F172A;
  flex: 1;
  text-align: left;
}

.model-actions {
  display: flex;
  align-items: center;
  gap: 5px;
}

.model-downloaded {
  font-size: 10px;
  font-weight: 600;
  color: var(--clr-accent);
}

.model-size {
  font-size: 10px;
  color: #94A3B8;
}

.model-btn {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.model-btn svg { width: 11px; height: 11px; }

.model-btn--dl {
  color: var(--clr-accent);
}

.model-btn--dl:hover:not(:disabled) {
  background: rgba(var(--clr-accent-rgb), 0.10);
}

.model-btn--dl:disabled {
  opacity: 0.30;
  cursor: not-allowed;
}

.model-btn--del {
  color: rgba(239, 68, 68, 0.45);
}

.model-btn--del:hover {
  color: #EF4444;
  background: rgba(239, 68, 68, 0.08);
}

.model-btn--load {
  color: var(--clr-accent);
}

.model-btn--load:hover:not(:disabled) {
  background: rgba(var(--clr-accent-rgb), 0.10);
}

.model-btn--load:disabled {
  opacity: 0.30;
  cursor: not-allowed;
}

.model-loaded {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 10px;
  font-weight: 700;
  color: var(--clr-accent);
}

.model-loading-badge {
  font-size: 10px;
  font-weight: 600;
  color: #D97706;
}

.model-btn--deleting {
  color: #EF4444;
  opacity: 0.60;
  cursor: not-allowed;
}

.model-btn--unload {
  color: rgba(156, 163, 175, 0.55); /* 灰色，低调 */
}

.model-btn--unload:hover:not(:disabled) {
  color: #6B7280;
  background: rgba(107, 114, 128, 0.10);
}

.model-btn--unload:disabled {
  opacity: 0.30;
  cursor: not-allowed;
}

.model-item--deleting {
  opacity: 0.50;
  pointer-events: none;
}

.spin-del {
  animation: spin-cw 0.7s linear infinite;
}

/* ===== 表单字段 ===== */
.field-stack {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.field__lbl {
  font-size: 11px;
  font-weight: 600;
  color: #475569;
  margin-bottom: 2px;
}

.field__input,
.field__select {
  width: 100%;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(0, 0, 0, 0.10);
  border-radius: 8px;
  color: #0F172A;
  font-size: 12px;
  padding: 7px 10px;
  outline: none;
  transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
  font-family: inherit;
}

.field__input::placeholder { color: #CBD5E1; }

.field__input:focus,
.field__select:focus {
  border-color: rgba(var(--clr-accent-rgb), 0.50);
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 0 0 3px rgba(var(--clr-accent-rgb), 0.10);
}

.field__select {
  appearance: none;
  -webkit-appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 12 12' fill='%2394A3B8'%3E%3Cpath d='M6 8L2 4h8L6 8z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 10px;
  padding-right: 26px;
  cursor: pointer;
}

.field__select option {
  background: #ffffff;
  color: #0F172A;
}

/* API Key 输入框 + 眼睛按钮 */
.field__pwd-wrap {
  position: relative;
  display: flex;
  align-items: center;
}

.field__pwd-wrap .field__input {
  padding-right: 34px;
}

.field__eye {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #CBD5E1;
  cursor: pointer;
  transition: color 0.15s;
}

.field__eye:hover { color: #475569; }
.field__eye svg { width: 13px; height: 13px; }

/* 范围滑块 */
.field__range {
  width: 100%;
  height: 4px;
  border-radius: 2px;
  background: rgba(var(--clr-accent-rgb), 0.12);
  outline: none;
  cursor: pointer;
  -webkit-appearance: none;
  appearance: none;
}

.field__range::-webkit-slider-thumb {
  -webkit-appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--clr-accent);
  cursor: pointer;
  box-shadow: 0 0 0 3px rgba(var(--clr-accent-rgb), 0.15), 0 1px 4px rgba(0,0,0,0.12);
}

.opacity-val {
  font-size: 11px;
  font-weight: 600;
  color: var(--clr-accent);
  font-variant-numeric: tabular-nums;
}

/* ===== 自定义 Toggle ===== */
.toggle-wrap {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  position: relative;
}

.toggle-input {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
}

.toggle-track {
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: rgba(0, 0, 0, 0.10);
  border: 1px solid rgba(0, 0, 0, 0.08);
  position: relative;
  transition: background 0.2s, border-color 0.2s;
}

.toggle-input:checked + .toggle-track {
  background: var(--clr-accent);
  border-color: var(--clr-accent);
}

.toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #ffffff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  transition: transform 0.2s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.toggle-input:checked + .toggle-track .toggle-thumb {
  transform: translateX(16px);
}

/* ===== 字段说明文字 ===== */
.field__hint {
  font-size: 10.5px;
  color: #94A3B8;
  line-height: 1.5;
  margin-top: 4px;
}

.field__hint strong {
  color: #475569;
  font-weight: 600;
}

/* ===== 测试连接 ===== */
.test-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.test-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  color: var(--clr-accent);
  background: rgba(var(--clr-accent-rgb), 0.08);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.20);
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
  flex-shrink: 0;
}

.test-btn svg { width: 12px; height: 12px; }

.test-btn:hover:not(:disabled) {
  color: var(--clr-accent-dark);
  background: rgba(var(--clr-accent-rgb), 0.14);
  border-color: rgba(var(--clr-accent-rgb), 0.35);
  box-shadow: 0 2px 8px rgba(var(--clr-accent-rgb), 0.12);
}

.test-btn:disabled {
  opacity: 0.50;
  cursor: not-allowed;
}

/* 旋转 loading 圆 */
.test-spin {
  animation: spin-cw 0.7s linear infinite;
  flex-shrink: 0;
}

.test-result {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  font-weight: 600;
  flex: 1;
  min-width: 0;
  word-break: break-all;
}

.test-ico { width: 10px; height: 10px; flex-shrink: 0; }

.test-result--ok  { color: var(--clr-accent); }
.test-result--err { color: #EF4444; }

/* ===== 危险操作按钮 ===== */
.danger-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  color: rgba(239, 68, 68, 0.70);
  background: rgba(239, 68, 68, 0.05);
  border: 1px solid rgba(239, 68, 68, 0.14);
  cursor: pointer;
  transition: color 0.15s, background 0.15s, border-color 0.15s;
  width: 100%;
  justify-content: center;
}

.danger-btn:hover {
  color: #EF4444;
  background: rgba(239, 68, 68, 0.10);
  border-color: rgba(239, 68, 68, 0.28);
}

/* ===== 主题选择器 ===== */
.theme-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
}

.theme-swatch {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 5px;
  cursor: pointer;
  padding: 6px 8px;
  border-radius: 10px;
  border: 1.5px solid transparent;
  transition: border-color 0.15s, background 0.15s;
}

.theme-swatch--on {
  border-color: rgba(var(--clr-accent-rgb), 0.40);
  background: rgba(var(--clr-accent-rgb), 0.06);
}

.theme-swatch:not(.theme-swatch--on):hover {
  background: rgba(0, 0, 0, 0.04);
}

.theme-dot {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.18), 0 0 0 1px rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
  transition: transform 0.15s;
}

.theme-swatch:hover .theme-dot {
  transform: scale(1.08);
}

.theme-check {
  width: 10px;
  height: 10px;
}

.theme-name {
  font-size: 10px;
  font-weight: 600;
  color: #94A3B8;
  letter-spacing: 0.02em;
}

.theme-swatch--on .theme-name {
  color: var(--clr-accent);
}

/* ===== 翻译用量标签 ===== */
.field__link {
  color: var(--clr-accent);
  text-decoration: underline;
  cursor: default;
}

.quota-tag {
  display: inline-flex;
  align-items: center;
  padding: 2px 9px;
  border-radius: 10px;
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.quota-tag--ok {
  background: rgba(var(--clr-accent-rgb), 0.10);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.22);
  color: var(--clr-accent);
}

.quota-tag--low {
  background: rgba(217, 119, 6, 0.10);
  border: 1px solid rgba(217, 119, 6, 0.30);
  color: #D97706;
}

.quota-tag--empty {
  background: rgba(220, 38, 38, 0.10);
  border: 1px solid rgba(220, 38, 38, 0.30);
  color: #DC2626;
}

/* ===== 权限提示框 ===== */
.perm-hint {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  padding: 8px 10px;
  border-radius: 9px;
  background: rgba(234, 179, 8, 0.07);
  border: 1px solid rgba(234, 179, 8, 0.22);
  font-size: 10.5px;
  color: #78350F;
  line-height: 1.55;
}

.perm-hint__icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  margin-top: 1px;
  color: #D97706;
}

.perm-hint strong {
  font-weight: 700;
  color: #92400E;
}

/* inline code 标签 */
.hint-code {
  display: inline;
  font-family: ui-monospace, 'SFMono-Regular', monospace;
  font-size: 10px;
  font-weight: 600;
  background: rgba(var(--clr-accent-rgb), 0.10);
  color: var(--clr-accent);
  border-radius: 4px;
  padding: 1px 4px;
}
</style>
