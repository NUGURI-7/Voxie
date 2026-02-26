<template>
  <!--
    app-shell: 纯裁切层，不做任何视觉效果
    backface-visibility:hidden + overflow:hidden + border-radius
    → 触发 WebKit 硬件合成路径，圆角 AA 光滑

    app-container: 玻璃效果层，不做裁切
    backdrop-filter 单独在合成层里运行，避免与裁切冲突
  -->
  <div class="app-shell">
    <div
      class="app-container"
      :style="{ '--window-opacity': appStore.settings.windowOpacity }"
    >
      <RouterView v-slot="{ Component }">
        <Transition name="page-slide">
          <component :is="Component" />
        </Transition>
      </RouterView>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { RouterView, useRouter } from 'vue-router'
import { useAppStore } from '@/stores/app'

const router = useRouter()
const appStore = useAppStore()

let unlistenNavigate: (() => void) | null = null

onMounted(async () => {
  await appStore.loadSettings()

  if (appStore.isTauri) {
    const { listen } = await import('@tauri-apps/api/event')
    unlistenNavigate = await listen('navigate-to-settings', () => {
      router.push('/settings')
    })
    await listen<number>('update-opacity', (event) => {
      appStore.settings.windowOpacity = event.payload
    })
  }
})

onUnmounted(() => {
  if (unlistenNavigate) unlistenNavigate()
})
</script>

<style scoped>
/* ─── 裁切层：只负责圆角裁切，无视觉效果 ─────────────────────────────
 *
 * 关键：backface-visibility:hidden + transform:translate3d(0,0,0)
 * 让 WebKit 把这个元素提升为独立 GPU 合成层，
 * 合成层上的 overflow:hidden+border-radius 走硬件 AA 路径，
 * 圆角边缘平滑。
 *
 * 不能在此元素上加 backdrop-filter：
 *   backdrop-filter 会捕获背后内容并重新合成，
 *   与裁切层同存会使 WebKit 产生合成冲突，强制退回 software rendering。
 * ─────────────────────────────────────────────────────────────────── */
.app-shell {
  position: fixed;
  inset: 0;
  border-radius: 22px;
  overflow: hidden;
  /* 硬件合成 + 背面不可见 = WebKit AA 圆角的标准触发组合 */
  transform: translate3d(0, 0, 0);
  -webkit-transform: translate3d(0, 0, 0);
  -webkit-backface-visibility: hidden;
  backface-visibility: hidden;
}

/* ─── 玻璃效果层：负责磨砂背景，不做任何裁切 ─────────────────────── */
.app-container {
  width: 100%;
  height: 100%;

  /* 液态玻璃核心 */
  background: rgba(var(--bg-window-tint), calc(var(--window-opacity, 0.82)));
  backdrop-filter: blur(48px) saturate(210%) brightness(1.03);
  -webkit-backdrop-filter: blur(48px) saturate(210%) brightness(1.03);

  /* 边框用 inset box-shadow 替代 border：
     inset shadow 跟随 border-radius 平滑过渡，比 border 更少锯齿 */
  box-shadow:
    inset 0 0    0 1px rgba(255, 255, 255, 0.80),   /* 外轮廓高光线 */
    inset 0 1.5px 0 0   rgba(255, 255, 255, 0.95),  /* 顶部镜面高光 */
    inset 0 -1px  0 0   rgba(0, 0, 0, 0.04);        /* 底部玻璃厚度感 */
}

/* ─── 页面切换动画 ─────────────────────────────────────────────────── */
.page-slide-enter-active {
  transition: opacity 0.18s ease, transform 0.18s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}
.page-slide-leave-active {
  transition: opacity 0.12s ease;
  position: absolute;
  width: 100%;
}
.page-slide-enter-from { opacity: 0; transform: translateX(14px); }
.page-slide-leave-to   { opacity: 0; }
</style>
