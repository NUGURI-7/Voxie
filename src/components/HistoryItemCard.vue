<template>
  <!-- 历史记录条目卡片 -->
  <div
    class="hc"
    :class="{ 'hc--latest': isLatest }"
    @click="$emit('click')"
  >
    <!-- 最新条目左侧蓝色高亮线 -->
    <div v-if="isLatest" class="hc__accent"></div>

    <!-- 文字内容 -->
    <p class="hc__text" :class="isLatest ? 'hc__text--bright' : 'hc__text--dim'">
      {{ item.text }}
    </p>

    <!-- 底部元数据 -->
    <div class="hc__meta">
      <span class="hc__time">{{ formattedTime }}</span>
      <span class="hc__dur">{{ formattedDuration }}</span>
    </div>

    <!-- 悬浮操作按钮（hover 时显示）-->
    <div class="hc__actions">
      <!-- 翻译 -->
      <button class="hc__btn hc__btn--translate no-drag"
        @click.stop="$emit('translate')"
        title="翻译">
        <Languages :size="14" :stroke-width="2" />
      </button>

      <!-- 复制 -->
      <button class="hc__btn hc__btn--copy no-drag"
        @click.stop="$emit('click')"
        title="复制">
        <Copy :size="15" :stroke-width="2" />
      </button>

      <!-- 删除 -->
      <button class="hc__btn hc__btn--del no-drag"
        @click.stop="$emit('delete')"
        title="删除">
        <Trash2 :size="15" :stroke-width="2" />
      </button>
    </div>

  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { HistoryItem } from '@/stores/app'
import { Copy, Trash2, Languages } from 'lucide-vue-next'

const props = defineProps<{
  item: HistoryItem
  isLatest: boolean
}>()

defineEmits<{
  click: []
  delete: []
  translate: []
}>()

// 格式化相对时间
const formattedTime = computed(() => {
  const date = new Date(props.item.timestamp)
  const now  = new Date()
  const diffSec  = Math.floor((now.getTime() - date.getTime()) / 1000)
  const diffMin  = Math.floor(diffSec / 60)
  const diffHour = Math.floor(diffMin / 60)

  if (diffSec  < 60)   return '刚刚'
  if (diffMin  < 60)   return `${diffMin} 分钟前`
  if (diffHour < 24)   return `${diffHour} 小时前`
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', hour12: false })
})

// 格式化录音时长
const formattedDuration = computed(() => {
  const ms = props.item.durationMs
  if (ms < 1000) return `${ms}ms`
  return `${(ms / 1000).toFixed(1)}s`
})
</script>

<style scoped>
/* ===== 卡片容器 ===== */
.hc {
  position: relative;
  border-radius: 9px;
  padding: 8px 9px 7px 11px;
  overflow: hidden;
  cursor: pointer;
  transition: background 0.18s, border-color 0.18s, box-shadow 0.20s, transform 0.14s cubic-bezier(0.34, 1.56, 0.64, 1);
  /* 液态玻璃卡片：磨砂底 + 顶部内高光 */
  background: rgba(255, 255, 255, 0.72);
  border: 1px solid rgba(255, 255, 255, 0.65);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.90),
    0 1px 4px rgba(0, 0, 0, 0.04);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  overflow: hidden;
}

.hc--latest {
  background: rgba(220, 252, 231, 0.88);
  border-color: rgba(22, 163, 74, 0.28);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.95),
    0 2px 12px rgba(22, 163, 74, 0.12);
}

.hc:not(.hc--latest):hover {
  background: rgba(255, 255, 255, 0.96);
  border-color: rgba(255, 255, 255, 0.92);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.98),
    0 8px 24px rgba(0, 0, 0, 0.14),
    0 2px 6px rgba(0, 0, 0, 0.08);
  transform: translateY(-2px);
}

.hc--latest:hover {
  background: rgba(204, 251, 241, 0.98);
  border-color: rgba(22, 163, 74, 0.45);
  box-shadow:
    inset 0 1px 0 rgba(255, 255, 255, 0.98),
    0 8px 26px rgba(22, 163, 74, 0.24),
    0 2px 8px rgba(22, 163, 74, 0.14);
  transform: translateY(-2px);
}

/* ===== 左侧高亮线（非最新条目 hover 时淡入）===== */
.hc:not(.hc--latest)::before {
  content: '';
  position: absolute;
  left: 0;
  top: 5px;
  bottom: 5px;
  width: 3px;
  border-radius: 2px;
  background: rgba(var(--clr-accent-rgb), 0.40);
  opacity: 0;
  transition: opacity 0.18s;
}

.hc:not(.hc--latest):hover::before { opacity: 1; }

/* ===== 左侧高亮线（最新条目固定显示）===== */
.hc__accent {
  position: absolute;
  left: 0;
  top: 5px;
  bottom: 5px;
  width: 3px;
  border-radius: 2px;
  background: linear-gradient(to bottom, #22C55E, #16A34A);
  box-shadow: 0 0 8px rgba(22, 163, 74, 0.45);
}

/* ===== 文字内容 ===== */
.hc__text {
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
  font-size: 12px;
  line-height: 1.55;
  padding-right: 90px;
}

.hc__text--bright { color: #0F172A; }
.hc__text--dim    { color: #334155; }

/* ===== 元数据行 ===== */
.hc__meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 5px;
}

.hc__time,
.hc__dur {
  font-size: 10px;
  font-weight: 500;
  color: #94A3B8;
}

/* ===== 操作按钮（hover 显示）===== */
.hc__actions {
  position: absolute;
  right: 6px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  flex-direction: row;
  gap: 3px;
  opacity: 0;
  transition: opacity 0.15s;
}

.hc:hover .hc__actions { opacity: 1; }

.hc__btn {
  width: 26px;
  height: 26px;
  border-radius: 7px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}

.hc__btn svg { width: 13px; height: 13px; }

.hc__btn--translate {
  color: var(--clr-accent);
}

.hc__btn--translate:hover {
  color: var(--clr-accent-dark);
  background: rgba(var(--clr-accent-rgb), 0.10);
}

.hc__btn--copy {
  color: #64748B;
}

.hc__btn--copy:hover {
  color: #0F172A;
  background: rgba(0, 0, 0, 0.08);
}

.hc__btn--del {
  color: rgba(239, 68, 68, 0.72);
}

.hc__btn--del:hover {
  color: #EF4444;
  background: rgba(239, 68, 68, 0.10);
}
</style>
