<template>
  <div class="fw h-full flex flex-col">

    <!-- ===== 标题栏（始终可见，收起时仅显示此栏）===== -->
    <div class="fw-bar select-none" data-tauri-drag-region>

      <!-- 左侧：状态指示器 -->
      <div class="fw-bar__left" data-tauri-drag-region>
        <div class="fw-dot" :class="dotClass" data-tauri-drag-region>
          <span v-if="appStore.isRecording" class="fw-dot__ring"></span>
          <span v-if="!appStore.isRecording && !appStore.isProcessing"
            class="fw-dot__logo" aria-hidden="true"></span>
          <Mic v-else-if="appStore.isRecording" class="fw-dot__icon" :size="15" :stroke-width="2" />
          <Loader2 v-else class="fw-dot__icon fw-dot__spin" :size="15" :stroke-width="2.5" />
        </div>
      </div>

      <!-- 右侧：设置 + 折叠 -->
      <div class="fw-bar__right">
        <button class="fw-btn no-drag" @click="openSettings" title="设置">
          <Settings2 :size="16" :stroke-width="2" />
        </button>
        <button class="fw-btn no-drag" @click="toggleCollapse" :title="isCollapsed ? '展开窗口' : '折叠窗口'">
          <ChevronUp v-if="isCollapsed" :size="16" :stroke-width="2.5" />
          <ChevronDown v-else :size="16" :stroke-width="2.5" />
        </button>
      </div>
    </div>

    <!-- ===== 展开区域：模式 Tab + 内容（收起时隐藏）===== -->
    <Transition name="slide-up">
      <div v-if="!isCollapsed" class="fw-body">

        <!-- 模式切换 Tab -->
        <div class="fw-mode-tabs">
          <div class="fw-tab-track">
            <div class="fw-tab-pill" :class="`fw-tab-pill--${fwMode}`"></div>
            <button
              class="fw-tab no-drag"
              :class="{ 'fw-tab--on': fwMode === 'transcribe' }"
              @click="fwMode = 'transcribe'"
            >
              <Mic :size="11" :stroke-width="2.5" />
              转录
            </button>
            <button
              class="fw-tab no-drag"
              :class="{ 'fw-tab--on': fwMode === 'translate' }"
              @click="fwMode = 'translate'"
            >
              <Languages :size="11" :stroke-width="2.5" />
              翻译
            </button>
          </div>
        </div>

        <!-- Tab 内容区：mode="out-in" 避免两个视图同时过渡 -->
        <Transition name="fw-tab-fade" mode="out-in">

          <!-- 翻译视图 -->
          <TranslationView v-if="fwMode === 'translate'" key="translate" />

          <!-- 转录视图 -->
          <div v-else class="fw-transcribe-pane" key="transcribe">

            <!-- 历史记录内容区 -->
            <div class="fw-content">
              <div class="fw-list">

                <!-- 空状态 -->
                <div v-if="appStore.history.length === 0" class="fw-empty">
                  <div class="fw-empty__icon">
                    <svg viewBox="0 0 24 24" fill="none"
                      stroke="currentColor" stroke-width="1.3">
                      <path stroke-linecap="round" stroke-linejoin="round"
                        d="M12 18.75a6 6 0 006-6v-1.5m-6 7.5a6 6 0 01-6-6v-1.5
                           m6 7.5v3.75m-3.75 0h7.5M12 15.75a3 3 0 01-3-3V4.5a3 3 0
                           116 0v8.25a3 3 0 01-3 3z"/>
                    </svg>
                  </div>
                  <p class="fw-empty__hint">点击下方麦克风按钮开始录音</p>
                </div>

                <!-- 历史记录条目 -->
                <TransitionGroup name="item-slide" tag="div" class="fw-cards">
                  <HistoryItemCard
                    v-for="(item, index) in appStore.history"
                    :key="item.id"
                    :item="item"
                    :is-latest="index === 0"
                    @click="appStore.copyToClipboard(item.text)"
                    @delete="appStore.deleteHistoryItem(item.id)"
                    @translate="sendToTranslate(item.text)"
                  />
                </TransitionGroup>
              </div>

              <!-- 状态栏 -->
              <div class="fw-status">

                <!-- 录音中 -->
                <template v-if="appStore.isRecording">
                  <div class="fw-waves">
                    <span v-for="i in 4" :key="i" class="fw-wave"
                      :style="{ animationDelay: `${(i - 1) * 0.1}s` }">
                    </span>
                  </div>
                  <span class="fw-status__label fw-status__label--rec">录音中</span>
                  <span class="fw-status__extra">{{ recordingDuration }}</span>
                </template>

                <!-- 本地模型加载中 -->
                <template v-else-if="appStore.settings.mode === 'local' && appStore.modelStatus === 'loading'">
                  <Loader2 class="fw-status__spin" :size="12" :stroke-width="2.5" />
                  <span class="fw-status__label fw-status__label--load">模型加载中</span>
                </template>

                <!-- 识别中 -->
                <template v-else-if="appStore.isProcessing">
                  <div class="fw-dots">
                    <span v-for="i in 3" :key="i" class="fw-dot-anim"
                      :style="{ animationDelay: `${(i - 1) * 0.15}s` }">
                    </span>
                  </div>
                  <span class="fw-status__label fw-status__label--proc">识别中</span>
                </template>

                <!-- 空闲 -->
                <template v-else>
                  <span class="fw-badge">
                    {{ appStore.settings.mode === 'local' ? '本地' : '云端' }}
                  </span>
                  <span v-if="appStore.history.length" class="fw-status__extra">
                    {{ appStore.history.length }} 条
                  </span>
                </template>

                <span class="fw-brand">Voxie</span>
              </div>
            </div>

            <!-- 麦克风按钮区 -->
            <div class="fw-mic-footer">
              <button
                class="fw-mic-btn no-drag"
                :class="{
                  'fw-mic-btn--rec': appStore.isRecording,
                  'fw-mic-btn--disabled': appStore.isProcessing || appStore.modelStatus === 'loading',
                }"
                :disabled="appStore.isProcessing || appStore.modelStatus === 'loading'"
                :title="appStore.isRecording ? '点击停止录音' : '点击开始录音'"
                @click="toggleRecording"
              >
                <Mic v-if="!appStore.isRecording" :size="20" :stroke-width="2" />
                <Square v-else :size="16" fill="currentColor" :stroke-width="0" />
              </button>
            </div>

          </div>
        </Transition>

      </div>
    </Transition>

    <!-- ===== Toast 提示 ===== -->
    <Transition name="toast-up">
      <div v-if="appStore.toast" class="fw-toast-layer">
        <div class="fw-toast" :class="`fw-toast--${appStore.toast.type}`">
          <CheckCircle2 v-if="appStore.toast.type === 'success'" class="fw-toast__ico" :size="12" :stroke-width="2.5" />
          <Info v-else class="fw-toast__ico" :size="12" :stroke-width="2.5" />
          {{ appStore.toast.message }}
        </div>
      </div>
    </Transition>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'
import HistoryItemCard from '@/components/HistoryItemCard.vue'
import { Mic, Square, Settings2, ChevronDown, ChevronUp, Loader2, CheckCircle2, Info, Languages } from 'lucide-vue-next'
import TranslationView from './TranslationView.vue'

const router = useRouter()
const appStore = useAppStore()

// ===== 窗口模式（转录 / 翻译）=====
const fwMode = ref<'transcribe' | 'translate'>('transcribe')

// ===== 折叠状态 =====
const isCollapsed = computed({
  get: () => appStore.isCollapsed,
  set: (val) => { appStore.isCollapsed = val },
})

// ===== 录音计时 =====
const recordingStartTime = ref<number | null>(null)
const recordingDuration = ref('0:00')
let durationTimer: number | null = null

// ===== 状态点样式 =====
const dotClass = computed(() => {
  if (appStore.isRecording)  return 'fw-dot--rec'
  if (appStore.isProcessing) return 'fw-dot--proc'
  return 'fw-dot--idle'
})

// ===== Tauri 事件监听 =====
let unlistenNew: (() => void) | null = null

onMounted(async () => {
  await appStore.loadHistory()

  if (appStore.isTauri) {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenNew = await listen('new-transcription', () => {
      appStore.loadHistory()
    })
  }
})

onUnmounted(() => {
  if (unlistenNew) unlistenNew()
  if (durationTimer) clearInterval(durationTimer)
})

// ===== 录音控制 =====
async function startRecording() {
  await appStore.startRecording()
  recordingStartTime.value = Date.now()
  durationTimer = window.setInterval(updateDuration, 100)
}

async function stopRecording() {
  if (durationTimer) { clearInterval(durationTimer); durationTimer = null }
  recordingStartTime.value = null
  await appStore.stopRecording()
}

function updateDuration() {
  if (!recordingStartTime.value) return
  const sec = (Date.now() - recordingStartTime.value) / 1000
  const m = Math.floor(sec / 60)
  const s = Math.floor(sec % 60)
  recordingDuration.value = `${m}:${s.toString().padStart(2, '0')}`
  // 超过 60 秒自动停止
  if (sec >= 60) {
    stopRecording()
  }
}

// ===== 点击录音按钮：开始 / 停止 =====
async function toggleRecording() {
  if (appStore.isProcessing) return
  if (appStore.isRecording) {
    await stopRecording()
  } else {
    await startRecording()
  }
}

// ===== 折叠 / 展开（带缓动动画缩放 Tauri 窗口高度）=====
const COLLAPSED_W   = 320   // 收起宽度（与展开相同）
const COLLAPSED_H   = 46    // 收起高度（仅标题栏）
const EXPANDED_W    = 320   // 展开宽度
const EXPANDED_H    = 480   // 展开高度
const ANIM_DURATION = 320   // 动画时长 ms

let isAnimating = false

/** ease-in-out cubic 缓动函数 */
function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2
}

/**
 * 逐帧动画：窗口尺寸从 (fromW, fromH) → (toW, toH)
 */
async function animateWindowSize(fromW: number, fromH: number, toW: number, toH: number): Promise<void> {
  const { getCurrentWindow } = await import('@tauri-apps/api/window')
  const { LogicalSize }      = await import('@tauri-apps/api/dpi')
  const win  = getCurrentWindow()
  const t0   = performance.now()

  return new Promise((resolve) => {
    function frame(now: number) {
      const t = Math.min((now - t0) / ANIM_DURATION, 1)
      const e = easeInOutCubic(t)
      const w = Math.round(fromW + (toW - fromW) * e)
      const h = Math.round(fromH + (toH - fromH) * e)
      win.setSize(new LogicalSize(w, h)).catch(() => {})
      if (t < 1) {
        requestAnimationFrame(frame)
      } else {
        resolve()
      }
    }
    requestAnimationFrame(frame)
  })
}

async function toggleCollapse() {
  if (isAnimating) return
  isAnimating = true

  const willCollapse = !appStore.isCollapsed

  if (!appStore.isTauri) {
    appStore.isCollapsed = willCollapse
    isAnimating = false
    return
  }

  try {
    if (willCollapse) {
      // 收起：先更新状态（触发内容区 leave 动画），然后收缩窗口
      appStore.isCollapsed = true
      await animateWindowSize(EXPANDED_W, EXPANDED_H, COLLAPSED_W, COLLAPSED_H)
    } else {
      // 展开：先撑大窗口，然后更新状态（触发内容区 enter 动画）
      await animateWindowSize(COLLAPSED_W, COLLAPSED_H, EXPANDED_W, EXPANDED_H)
      appStore.isCollapsed = false
    }
  } catch (e) {
    console.warn('窗口动画失败:', e)
    appStore.isCollapsed = willCollapse
  } finally {
    isAnimating = false
  }
}

function openSettings() { router.push('/settings') }

// ===== 从转录页跳转翻译 =====
function sendToTranslate(text: string) {
  appStore.pendingTranslationText = text
  fwMode.value = 'translate'
}

</script>

<style scoped>
/* ===== 根容器 ===== */
.fw {
  border-radius: 16px;
  clip-path: inset(0 round 16px);
  position: relative;
}

/* ===== 展开内容容器 ===== */
.fw-body {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* ===== 标题栏 ===== */
.fw-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px;
  height: 46px;
  background: rgba(255, 255, 255, 0.62);
  backdrop-filter: blur(16px) saturate(180%);
  -webkit-backdrop-filter: blur(16px) saturate(180%);
  border-bottom: 1px solid rgba(255, 255, 255, 0.55);
  box-shadow:
    inset 0  1px 0 rgba(255, 255, 255, 0.95),
    inset 0 -1px 0 rgba(var(--clr-accent-rgb), 0.08),
    0 1px 8px rgba(0, 0, 0, 0.06);
  flex-shrink: 0;
}

.fw-bar__left { display: flex; align-items: center; gap: 7px; }
.fw-bar__right { display: flex; align-items: center; gap: 1px; }

/* ===== 状态指示点 ===== */
.fw-dot {
  position: relative;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.fw-dot--idle { background: transparent; }
.fw-dot--rec  { background: rgba(239, 68,  68,  0.12); }
.fw-dot--proc { background: rgba(59,  130, 246, 0.12); }

.fw-dot__ring {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: rgba(239, 68, 68, 0.40);
  animation: pulse-ring 1.3s ease-out infinite;
}

.fw-dot__icon {
  position: relative;
  z-index: 1;
  flex-shrink: 0;
}

.fw-dot__logo {
  display: inline-block;
  width: 22px;
  height: 22px;
  flex-shrink: 0;
  position: relative;
  z-index: 1;
  background-image: url('@/assets/voxie-logo.png');
  background-size: contain;
  background-repeat: no-repeat;
  background-position: center;
}

.fw-dot--idle .fw-dot__icon { color: var(--clr-accent); }
.fw-dot--rec  .fw-dot__icon { color: #EF4444; }
.fw-dot--proc .fw-dot__icon { color: #3B82F6; }

.fw-dot__spin { animation: spin-cw 0.9s linear infinite; }

/* ===== 控制按钮 ===== */
.fw-btn {
  width: 34px;
  height: 34px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748B;
  cursor: pointer;
  transition: color 0.15s, background 0.18s, box-shadow 0.18s, transform 0.1s;
}

.fw-btn:hover {
  color: #0F172A;
  background: rgba(255, 255, 255, 0.72);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08), inset 0 1px 0 rgba(255, 255, 255, 0.90);
}

.fw-btn:active {
  transform: scale(0.90);
  background: rgba(255, 255, 255, 0.50);
}

/* ===== 模式切换 Tab ===== */
.fw-mode-tabs {
  padding: 5px 8px 4px;
  background: rgba(255, 255, 255, 0.28);
  border-bottom: 1px solid rgba(255, 255, 255, 0.60);
  flex-shrink: 0;
}

.fw-tab-track {
  position: relative;
  display: flex;
  background: rgba(var(--clr-accent-rgb), 0.07);
  border-radius: 9px;
  padding: 2px;
}

.fw-tab-pill {
  position: absolute;
  top: 2px;
  bottom: 2px;
  left: 2px;
  width: calc(50% - 2px);
  border-radius: 7px;
  background: #fff;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.10), inset 0 1px 0 rgba(255, 255, 255, 0.90);
  transition: transform 0.22s cubic-bezier(0.34, 1.56, 0.64, 1);
  pointer-events: none;
}

.fw-tab-pill--transcribe { transform: translateX(0); }
.fw-tab-pill--translate  { transform: translateX(100%); }

.fw-tab {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 4px 10px;
  border-radius: 7px;
  font-size: 11px;
  font-weight: 600;
  color: #64748B;
  cursor: pointer;
  transition: color 0.15s;
  letter-spacing: 0.01em;
  position: relative;
  z-index: 1;
  background: none;
  border: none;
}

.fw-tab:hover { color: #334155; }
.fw-tab--on   { color: var(--clr-accent); }

/* ===== 转录面板（内含内容区 + 麦克风）===== */
.fw-transcribe-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

/* ===== 内容区域 ===== */
.fw-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

/* ===== 历史列表 ===== */
.fw-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--clr-accent-rgb), 0.18) transparent;
}

.fw-cards {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

/* ===== 空状态 ===== */
.fw-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  min-height: 80px;
  gap: 10px;
}

.fw-empty__icon {
  width: 52px;
  height: 52px;
  color: rgba(var(--clr-accent-rgb), 0.28);
}

.fw-empty__icon svg { width: 100%; height: 100%; }

.fw-empty__hint {
  font-size: 11.5px;
  color: #94A3B8;
  text-align: center;
  line-height: 1.6;
}

/* ===== 底部状态栏 ===== */
.fw-status {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 12px;
  height: 38px;
  border-top: 1px solid rgba(255, 255, 255, 0.70);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.90),
              0 -1px 0 rgba(var(--clr-accent-rgb), 0.06);
  background: rgba(255, 255, 255, 0.30);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  flex-shrink: 0;
}

.fw-status__label {
  font-size: 11.5px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.fw-status__label--rec  { color: #EF4444; }
.fw-status__label--proc { color: #3B82F6; }
.fw-status__label--load { color: #D97706; }

.fw-status__spin {
  color: #D97706;
  animation: spin-cw 0.9s linear infinite;
  flex-shrink: 0;
}

.fw-status__extra {
  font-size: 11px;
  font-weight: 500;
  color: #94A3B8;
  font-variant-numeric: tabular-nums;
}

.fw-brand {
  margin-left: auto;
  font-size: 10.5px;
  font-weight: 800;
  color: rgba(var(--clr-accent-rgb), 0.40);
  letter-spacing: 0.10em;
  text-transform: uppercase;
  user-select: none;
}

/* ===== 录音波形 ===== */
.fw-waves {
  display: flex;
  align-items: center;
  gap: 2.5px;
}

.fw-wave {
  display: block;
  width: 2.5px;
  height: 13px;
  background: #EF4444;
  border-radius: 2px;
  transform-origin: center;
  animation: wave-dance 0.65s ease-in-out infinite;
}

/* ===== 识别中点动画 ===== */
.fw-dots {
  display: flex;
  align-items: center;
  gap: 3px;
}

.fw-dot-anim {
  display: block;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #3B82F6;
  animation: dot-bounce 0.75s ease-in-out infinite;
}

/* ===== 模式徽章 ===== */
.fw-badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 9px;
  border-radius: 10px;
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.03em;
  background: rgba(var(--clr-accent-rgb), 0.10);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.22);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.70);
  color: var(--clr-accent);
}

/* ===== 麦克风按钮底部区域 ===== */
.fw-mic-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 10px 0 13px;
  border-top: 1px solid rgba(255, 255, 255, 0.65);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.85);
  background: rgba(255, 255, 255, 0.22);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  flex-shrink: 0;
}

.fw-mic-btn {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  flex-shrink: 0;
  background: rgba(var(--clr-accent-rgb), 0.12);
  border: 1.5px solid rgba(var(--clr-accent-rgb), 0.32);
  color: var(--clr-accent);
  transition: background 0.18s, border-color 0.18s, box-shadow 0.18s, transform 0.12s cubic-bezier(0.34, 1.56, 0.64, 1);
  box-shadow:
    0 2px 12px rgba(var(--clr-accent-rgb), 0.20),
    inset 0 1.5px 0 rgba(255, 255, 255, 0.75);
}

.fw-mic-btn:hover:not(:disabled) {
  background: rgba(var(--clr-accent-rgb), 0.18);
  border-color: rgba(var(--clr-accent-rgb), 0.55);
  transform: scale(1.10);
  box-shadow:
    0 6px 22px rgba(var(--clr-accent-rgb), 0.35),
    inset 0 1.5px 0 rgba(255, 255, 255, 0.85);
}

.fw-mic-btn:active:not(:disabled) {
  transform: scale(0.94);
  box-shadow: 0 2px 8px rgba(var(--clr-accent-rgb), 0.18);
}

.fw-mic-btn--rec {
  background: rgba(239, 68, 68, 0.13);
  border-color: rgba(239, 68, 68, 0.42);
  color: #EF4444;
  animation: mic-pulse 1.2s ease-in-out infinite;
  box-shadow:
    0 2px 12px rgba(239, 68, 68, 0.22),
    inset 0 1.5px 0 rgba(255, 255, 255, 0.65);
}

@keyframes mic-pulse {
  0%, 100% {
    box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.35),
                inset 0 1.5px 0 rgba(255, 255, 255, 0.65);
  }
  50% {
    box-shadow: 0 0 0 10px rgba(239, 68, 68, 0),
                inset 0 1.5px 0 rgba(255, 255, 255, 0.65);
  }
}

.fw-mic-btn--disabled {
  opacity: 0.38;
  cursor: not-allowed;
}

/* ===== Toast ===== */
.fw-toast-layer {
  position: absolute;
  bottom: 50px;
  left: 10px;
  right: 10px;
  display: flex;
  justify-content: center;
  pointer-events: none;
  z-index: 50;
}

.fw-toast {
  display: flex;
  align-items: flex-start;
  gap: 7px;
  padding: 9px 13px;
  border-radius: 12px;
  font-size: 11.5px;
  font-weight: 600;
  line-height: 1.5;
  word-break: break-word;
  max-width: 100%;
  letter-spacing: 0.01em;
  backdrop-filter: blur(28px) saturate(200%);
  -webkit-backdrop-filter: blur(28px) saturate(200%);
  border: 1px solid rgba(255, 255, 255, 0.30);
}

.fw-toast--success {
  background: linear-gradient(
    140deg,
    rgba(134, 239, 172, 0.72) 0%,
    rgba(34,  197, 94,  0.68) 100%
  );
  box-shadow:
    0 6px 20px rgba(22, 163, 74, 0.22),
    inset 0 1.5px 0 rgba(255, 255, 255, 0.60);
  color: #14532D;
}

.fw-toast--error {
  background: linear-gradient(
    140deg,
    rgba(253, 164, 175, 0.72) 0%,
    rgba(244,  63,  94, 0.68) 100%
  );
  box-shadow:
    0 6px 20px rgba(225, 29, 72, 0.20),
    inset 0 1.5px 0 rgba(255, 255, 255, 0.55);
  color: #7f1d1d;
}

.fw-toast--info {
  background: linear-gradient(
    140deg,
    rgba(147, 197, 253, 0.72) 0%,
    rgba(59,  130, 246, 0.68) 100%
  );
  box-shadow:
    0 6px 20px rgba(37, 99, 235, 0.20),
    inset 0 1.5px 0 rgba(255, 255, 255, 0.55);
  color: #1e3a8a;
}

.fw-toast__ico {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
  margin-top: 2px;
}
</style>
