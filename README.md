# Cobweb

Cobweb PC 端桌面应用，基于 Rust、Tauri 2 和 Vue 3 开发。

## 目录说明

```text
cobweb/
├── auth/        认证相关 Rust crate，目前保留基础模板，供后续登录、授权或身份校验能力扩展
├── common/      通用 Rust crate，目前保留基础模板，适合放置跨模块共享的基础类型和工具函数
├── control/     PC 控制能力 Rust crate，封装 Windows 系统控制、音频控制和媒体控制等底层调用
├── discovery/   设备发现相关 Rust crate，目前保留基础模板，供后续局域网发现能力扩展
├── scheduler/   定时任务 Rust crate，定义任务来源、任务模型、时间戳和功能命令描述逻辑
├── service/     业务服务 Rust crate，定义功能分组、功能命令、执行结果，并串联 control 提供的系统能力
├── share/       共享能力 Rust crate，目前保留基础模板，供后续跨端共享、数据交换等能力扩展
├── storage/     存储相关 Rust crate，目前保留基础模板，供后续持久化能力扩展
├── transport/   传输通信 Rust crate，目前保留基础模板，供后续网络传输协议或连接能力扩展
├── ui/          PC 桌面端 Tauri + Vue 项目，包含桌面端前端源码、Tauri 后端源码、托盘、设备、定时任务和本地服务入口
└── web/         局域网 Web 控制台 Vue 项目，构建产物会被 PC 端 Tauri/Axum 服务托管
```

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
