# Cobweb

Cobweb PC 端桌面应用，基于 Rust、Tauri 2 和 Vue 3 开发。

## 开发

```bash
cd ui
pnpm install
pnpm tauri dev
```

## 构建

```bash
cd ui
pnpm install
pnpm tauri build
```

GitHub Actions 会在 `main` 分支构建 Windows 安装包，并生成 Tauri updater 所需的 `latest.json`。
