<template>
  <div class="tv">

    <!-- ===== 语言选择行 ===== -->
    <div class="tv-langs">
      <div class="tv-lang-pills">
        <button
          v-for="lang in langs"
          :key="lang.value"
          class="tv-lang-pill no-drag"
          :class="{ 'tv-lang-pill--on': srcLang === lang.value }"
          @click="srcLang = lang.value"
        >{{ lang.label }}</button>
      </div>

      <button class="tv-swap no-drag" @click="swapLangs" title="交换语言">
        <ArrowLeftRight :size="13" :stroke-width="2.5" />
      </button>

      <div class="tv-lang-pills">
        <button
          v-for="lang in langs"
          :key="lang.value"
          class="tv-lang-pill no-drag"
          :class="{ 'tv-lang-pill--on': dstLang === lang.value }"
          @click="dstLang = lang.value"
        >{{ lang.label }}</button>
      </div>
    </div>

    <!-- ===== 源文本 ===== -->
    <div class="tv-src-wrap">
      <textarea
        ref="textareaEl"
        class="tv-textarea no-drag"
        v-model="srcText"
        placeholder="输入要翻译的文字…"
        maxlength="500"
        @keydown.meta.enter.prevent="doTranslate"
        @input="resizeTextarea"
      />
      <span class="tv-char-count">{{ srcText.length }}/500</span>
      <button v-if="srcText" class="tv-clear-btn no-drag" @click="clearAll" title="清空">
        <X :size="11" :stroke-width="2.5" />
      </button>
    </div>

    <!-- ===== 分隔线 ===== -->


    <!-- ===== 翻译结果 ===== -->
    <div class="tv-result-wrap">
      <div v-if="isTranslating" class="tv-loading">
        <Loader2 :size="14" :stroke-width="2.5" class="tv-spin" />
        <span>翻译中…</span>
      </div>
      <div v-else-if="errorMsg" class="tv-error">{{ errorMsg }}</div>
      <div v-else-if="dstText" class="tv-result-text">{{ dstText }}</div>
      <div v-else class="tv-result-empty">
        <Languages :size="22" :stroke-width="1.5" class="tv-empty-icon" />
        <span>翻译结果将显示在这里</span>
      </div>
    </div>

    <!-- ===== 底部操作行 ===== -->
    <div class="tv-footer">
      <!-- 今日用量 -->
      <span class="tv-quota" :class="quotaClass">{{ quotaText }}</span>

      <div class="tv-footer-btns">
        <button
          class="tv-icon-btn no-drag"
          @click="pasteFromClipboard"
          title="粘贴剪贴板内容"
        >
          <ClipboardPaste :size="13" :stroke-width="2" />
        </button>

        <button
          v-if="dstText && !isTranslating"
          class="tv-icon-btn no-drag"
          @click="copyResult"
          title="复制翻译结果"
        >
          <Copy :size="13" :stroke-width="2" />
        </button>

        <button
          class="tv-translate-btn no-drag"
          :disabled="!canTranslate"
          @click="doTranslate"
        >
          <Loader2 v-if="isTranslating" :size="12" :stroke-width="2.5" class="tv-spin" />
          <Languages v-else :size="13" :stroke-width="2.5" />
          翻译
        </button>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useAppStore } from '@/stores/app'
import type { TranslationLang } from '@/stores/app'
import { ArrowLeftRight, X, Loader2, Copy, Languages, ClipboardPaste } from 'lucide-vue-next'

const appStore = useAppStore()

// ===== Textarea 自动高度 =====
const textareaEl = ref<HTMLTextAreaElement | null>(null)

function resizeTextarea() {
  const el = textareaEl.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = Math.min(el.scrollHeight, 300) + 'px'
}

const langs = [
  { value: 'zh-hans' as TranslationLang, label: '简' },
  { value: 'zh-hant' as TranslationLang, label: '繁' },
  { value: 'en'      as TranslationLang, label: 'EN' },
]

// ===== 状态 =====
const srcLang = ref<TranslationLang>('zh-hans')
const dstLang = ref<TranslationLang>('en')
const srcText = ref('')
const dstText = ref('')
const isTranslating = ref(false)
const errorMsg = ref('')

// ===== 计算属性 =====
const canTranslate = computed(() =>
  srcText.value.trim().length > 0 &&
  !isTranslating.value &&
  srcLang.value !== dstLang.value
)

const quotaText = computed(() => {
  const u = appStore.translationUsage
  if (u.usedToday === 0 && !u.hasKey) return '每日 1000 次免费'
  const remaining = Math.max(0, u.limitToday - u.usedToday)
  return `今日剩余 ${remaining} 次`
})

const quotaClass = computed(() => {
  const u = appStore.translationUsage
  const remaining = u.limitToday - u.usedToday
  if (remaining <= 0) return 'tv-quota--empty'
  if (remaining < u.limitToday * 0.1) return 'tv-quota--low'
  return ''
})

// ===== 自动翻译 =====
let autoTimer: ReturnType<typeof setTimeout> | null = null

function scheduleAutoTranslate(delay: number) {
  if (autoTimer !== null) clearTimeout(autoTimer)
  autoTimer = setTimeout(() => {
    autoTimer = null
    if (canTranslate.value) doTranslate()
  }, delay)
}

// 文本变化：800ms 防抖，清空时立即清结果
watch(srcText, (val) => {
  nextTick(resizeTextarea)
  if (!val.trim()) {
    if (autoTimer !== null) { clearTimeout(autoTimer); autoTimer = null }
    dstText.value = ''
    errorMsg.value = ''
    return
  }
  scheduleAutoTranslate(800)
})

// 语言切换：300ms（有文本时触发）
watch([srcLang, dstLang], () => {
  if (!srcText.value.trim()) return
  scheduleAutoTranslate(300)
})

// 从转录页跳转过来时，接收待翻译文本并立即触发翻译
// immediate: true 确保组件挂载时若已有待翻译文本能立即消费
watch(() => appStore.pendingTranslationText, (text) => {
  if (!text) return
  srcText.value = text
  dstText.value = ''
  errorMsg.value = ''
  appStore.pendingTranslationText = ''
  nextTick(() => {
    resizeTextarea()
    scheduleAutoTranslate(100)
  })
}, { immediate: true })

onUnmounted(() => {
  if (autoTimer !== null) clearTimeout(autoTimer)
})

// ===== 动作 =====
function swapLangs() {
  const tmp = srcLang.value
  srcLang.value = dstLang.value
  dstLang.value = tmp
  // 同时交换文本
  const t = srcText.value
  srcText.value = dstText.value
  dstText.value = t
  errorMsg.value = ''
}

function clearAll() {
  srcText.value = ''
  dstText.value = ''
  errorMsg.value = ''
}

async function copyResult() {
  await appStore.copyToClipboard(dstText.value)
}

async function pasteFromClipboard() {
  try {
    let text = ''
    if (appStore.isTauri) {
      const { readText } = await import('@tauri-apps/plugin-clipboard-manager')
      text = (await readText()) ?? ''
    } else {
      text = await navigator.clipboard.readText()
    }
    if (text) {
      srcText.value = text
      nextTick(resizeTextarea)
    }
  } catch (e) {
    console.warn('读取剪贴板失败:', e)
  }
}

async function doTranslate() {
  if (!canTranslate.value) return
  isTranslating.value = true
  dstText.value = ''
  errorMsg.value = ''
  try {
    dstText.value = await appStore.translateText(srcText.value, srcLang.value, dstLang.value)
    appStore.getTranslationUsage()
  } catch (e) {
    errorMsg.value = String(e)
  } finally {
    isTranslating.value = false
  }
}

onMounted(() => {
  appStore.getTranslationUsage()
})
</script>

<style scoped>
/* ===== 根容器 ===== */
.tv {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
  padding: 8px 10px 0;
  gap: 0;
}

/* ===== 语言选择行 ===== */
.tv-langs {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
  padding-bottom: 8px;
}

.tv-lang-pills {
  flex: 1;
  display: flex;
  background: rgba(var(--clr-accent-rgb), 0.07);
  border-radius: 8px;
  padding: 2px;
  gap: 1px;
}

.tv-lang-pill {
  flex: 1;
  height: 24px;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 700;
  color: #64748B;
  cursor: pointer;
  transition: color 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
}

.tv-lang-pill:hover:not(.tv-lang-pill--on) { color: #334155; }

.tv-lang-pill--on {
  background: #fff;
  color: var(--clr-accent);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.10), inset 0 1px 0 rgba(255, 255, 255, 0.90);
  border-radius: 5px;
}

.tv-swap {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--clr-accent);
  background: rgba(var(--clr-accent-rgb), 0.08);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.20);
  flex-shrink: 0;
  cursor: pointer;
  transition: background 0.15s, transform 0.15s;
}

.tv-swap:hover {
  background: rgba(var(--clr-accent-rgb), 0.16);
  transform: scale(1.08);
}

.tv-swap:active { transform: scale(0.92) rotate(180deg); }

/* ===== 源文本框 ===== */
.tv-src-wrap {
  position: relative;
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.tv-textarea {
  resize: none;
  width: 100%;
  min-height: 105px;
  max-height: 300px;
  overflow-y: auto;
  padding: 8px 10px 22px;
  border-radius: 10px;
  font-size: 12.5px;
  line-height: 1.55;
  color: #0F172A;
  background: rgba(255, 255, 255, 0.62);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.18);
  outline: none;
  font-family: inherit;
  transition: border-color 0.15s, background 0.15s;
}

.tv-textarea:focus {
  border-color: rgba(var(--clr-accent-rgb), 0.50);
  background: rgba(255, 255, 255, 0.85);
}

.tv-textarea::placeholder { color: #94A3B8; }

.tv-char-count {
  position: absolute;
  left: 10px;
  bottom: 9px;
  font-size: 10px;
  color: rgba(148, 163, 184, 0.65);
  font-variant-numeric: tabular-nums;
  pointer-events: none;
  user-select: none;
}

.tv-clear-btn {
  position: absolute;
  right: 6px;
  bottom: 6px;
  width: 20px;
  height: 20px;
  border-radius: 5px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #94A3B8;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
  background: rgba(255, 255, 255, 0.75);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
}

.tv-clear-btn:hover { color: #475569; background: rgba(255, 255, 255, 0.95); }



/* ===== 翻译结果区 ===== */
.tv-result-wrap {
  flex: 1;
  min-height: 0;
  padding: 8px 10px 6px;
  border-radius: 10px;
  margin-top: 4px;
  background: rgba(var(--clr-accent-rgb), 0.07);
  border: 1px solid rgba(var(--clr-accent-rgb), 0.14);
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(var(--clr-accent-rgb), 0.15) transparent;
}

.tv-loading {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--clr-accent);
  font-size: 12px;
  font-weight: 600;
}

.tv-result-text {
  font-size: 12.5px;
  line-height: 1.60;
  color: #0F172A;
  word-break: break-word;
  white-space: pre-wrap;
}

.tv-result-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 7px;
}

.tv-empty-icon {
  color: rgba(var(--clr-accent-rgb), 0.25);
}

.tv-result-empty span {
  font-size: 11.5px;
  color: #94A3B8;
}

.tv-error {
  font-size: 11.5px;
  color: #DC2626;
  line-height: 1.55;
}

/* ===== 底部操作行 ===== */
.tv-footer {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 0 10px;
  flex-shrink: 0;
}

.tv-quota {
  font-size: 10.5px;
  color: #94A3B8;
  font-variant-numeric: tabular-nums;
}

.tv-quota--low   { color: #D97706; font-weight: 600; }
.tv-quota--empty { color: #DC2626; font-weight: 600; }

.tv-footer-btns {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 5px;
}

.tv-icon-btn {
  width: 28px;
  height: 28px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #64748B;
  cursor: pointer;
  transition: color 0.15s, background 0.15s;
}

.tv-icon-btn:hover { color: #0F172A; background: rgba(255,255,255,0.65); }

.tv-translate-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 12px;
  height: 28px;
  border-radius: 8px;
  font-size: 11.5px;
  font-weight: 700;
  color: #fff;
  background: var(--clr-accent);
  border: 1px solid rgba(0, 0, 0, 0.10);
  cursor: pointer;
  transition: opacity 0.15s, transform 0.12s, box-shadow 0.15s;
  box-shadow: 0 2px 8px rgba(var(--clr-accent-rgb), 0.35);
}

.tv-translate-btn:hover:not(:disabled) {
  opacity: 0.88;
  transform: translateY(-1px);
  box-shadow: 0 4px 14px rgba(var(--clr-accent-rgb), 0.45);
}

.tv-translate-btn:active:not(:disabled) {
  transform: translateY(0) scale(0.96);
}

.tv-translate-btn:disabled {
  opacity: 0.38;
  cursor: not-allowed;
  box-shadow: none;
}

/* ===== 动画 ===== */
.tv-spin { animation: spin-cw 0.9s linear infinite; }

@keyframes spin-cw {
  from { transform: rotate(0deg); }
  to   { transform: rotate(360deg); }
}
</style>
