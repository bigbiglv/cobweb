# Cobweb Web 控制台

这个目录是局域网 Web 控制台的独立 Vue 3 + Vite 项目。

PC 端 Tauri/Axum 负责托管 `dist` 构建产物，并提供 `/web/api/*`、`/web/ws`。Web 控制台自身维护页面状态、实时同步和交互逻辑，避免和 PC 桌面 App 的 Vue 项目混在一起。

## 命令

```bash
npm install
npm run dev
npm run build
```

PC 打包时会通过 `ui` 的 `build:web` 脚本先构建本项目，再由 Rust 在编译期嵌入 `dist/index.html`、`dist/assets/main.js` 和 `dist/assets/style.css`。

