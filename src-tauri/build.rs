// build.rs - Tauri 构建脚本
// 这个文件在编译时运行，用于生成 Tauri 所需的资源

fn main() {
    // tauri-build 会处理以下事项：
    // 1. 生成 tauri.conf.json 中定义的资源绑定
    // 2. 设置正确的链接标志
    // 3. 在 macOS 上处理代码签名相关配置
    tauri_build::build()
}
