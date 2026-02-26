// main.ts - Vue 应用入口
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'
import App from './App.vue'

// 导入全局样式（Tailwind CSS v4 + daisyUI）
import './assets/main.css'

// ===== 路由配置 =====
// 悬浮窗使用 memory history（不需要 URL 导航栏）
const router = createRouter({
  history: createMemoryHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('./views/FloatingWindow.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('./views/Settings.vue'),
    },
  ],
})

// ===== 创建应用 =====
const app = createApp(App)

// 注册 Pinia（Vue 状态管理）
app.use(createPinia())

// 注册路由
app.use(router)

// 挂载到 #app 元素
app.mount('#app')
