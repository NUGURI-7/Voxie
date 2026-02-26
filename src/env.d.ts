/// <reference types="vite/client" />

// 声明 .vue 文件的类型
// 没有这个文件，TypeScript 会不认识 .vue 文件
declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<{}, {}, any>
  export default component
}
