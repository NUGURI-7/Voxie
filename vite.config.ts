import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [
    vue(),
    tailwindcss(),
  ],

  // 路径别名：@ 指向 src 目录
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Vite 的开发服务器配置
  server: {
    port: 5173,
    strictPort: true,
    // 监听 Tauri CLI 的事件
    watch: {
      // 在监视 Tauri 应用时，这些目录的变化也触发热重载
      ignored: ["**/src-tauri/**"],
    },
  },

  // 防止 Vite 混淆 Tauri 的命令
  envPrefix: ["VITE_", "TAURI_ENV_*", "TAURI_PLATFORM", "TAURI_ARCH", "TAURI_FAMILY", "TAURI_DEBUG"],

  build: {
    // Tauri 在 Windows 上使用 Chromium，在 macOS/Linux 上使用 WebKit
    // WebKit 不支持所有现代 JS 特性，需要设置合适的 target
    target: process.env.TAURI_ENV_PLATFORM == "windows"
      ? "chrome105"
      : "safari13",
    // 不压缩源码（Tauri 发布时会自己压缩）
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    // 生成 source map 用于调试
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      output: {
        // 代码分割：将 vendor 库单独打包
        manualChunks: {
          vendor: ["vue", "vue-router", "pinia"],
        },
      },
    },
  },
}));
