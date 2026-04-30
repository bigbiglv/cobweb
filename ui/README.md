# Cobweb PC (Tauri + Vue)

Cobweb PC 端桌面应用，基于 Tauri 2.0 和 Vue 3 开发。

## 开发环境准备

1. **Rust**: 确保已安装 [Rust 编译环境](https://www.rust-lang.org/learn/get-started)。
2. **Node.js**: 建议使用最新 LTS 版本。
3. **WebView2 (Windows)**: Windows 用户需确保已安装 WebView2 运行时。

## 运行项目

在 `ui` 目录下执行：

1. **安装依赖**:
   ```bash
   npm install
   ```

2. **启动 Tauri 调试模式 (推荐)**:
   这将同时启动前端 Vite 服务和 Rust 后端，并打开一个桌面窗口。
   ```bash
   npm run tauri dev
   ```

3. **仅启动前端预览**:
   ```bash
   npm run dev
   ```

## 打包发布

1. **生成各平台安装包**:
   ```bash
   npm run tauri build
   ```

打包后的文件位于 `src-tauri/target/release/bundle/` 目录下。

## 项目结构

- `src`: 前端 Vue 源码
- `src-tauri`: Rust 后端源码及 Tauri 配置
- `src-tauri/src/peripherals.rs`: 外设控制逻辑
