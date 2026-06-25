<p align="center">
  <img src="logo.svg" width="96" alt="ServiceCompass Logo" />
</p>

<h1 align="center">ServiceCompass</h1>

<p align="center">
  轻量级自托管服务导航与状态监控面板，面向 Homelab、NAS 和个人服务器。
</p>

<p align="center">
  <a href="https://github.com/ydfk/service-compass/actions"><img alt="GitHub Actions" src="https://img.shields.io/github/actions/workflow/status/ydfk/service-compass/docker-publish.yml?branch=main&label=build"></a>
  <a href="https://github.com/ydfk/service-compass/releases"><img alt="Release" src="https://img.shields.io/github/v/release/ydfk/service-compass?display_name=tag"></a>
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green">
  <img alt="Docker" src="https://img.shields.io/badge/docker-ready-2496ed">
  <img alt="Rust" src="https://img.shields.io/badge/backend-rust%20%2B%20axum-f74c00">
  <img alt="Vue" src="https://img.shields.io/badge/frontend-vue%203-42b883">
</p>

## 它是什么？

ServiceCompass 把“服务”放在中心：你可以先维护一个清爽的服务首页，再按需关联 Docker 容器、启用 HTTP/关键字/Docker/证书等监控和通知。

它不是 Uptime Kuma 的完整替代品，也不打算变成复杂监控平台。它更像一个适合内网自托管环境的服务罗盘：先让服务好找，再让异常好发现。

## 亮点

- 服务首页：分组、排序、大小卡片、图标、登录后直接编辑
- 监控能力：HTTP/HTTPS、HTTP 关键字、Docker Healthcheck、DNS、HTTPS 证书到期
- 监控日志：按分组和服务查看状态条、最近检查日志和详细历史
- Docker 辅助：扫描候选容器，手动确认后再添加服务和关联监控
- 通知通道：Bark、Webhook、Synology Chat，支持按服务范围生效
- 单镜像部署：Rust 后端托管 Vue 前端 dist，SQLite 数据落盘
- 管理功能：账号密码登录、系统日志、亮/暗主题、版本更新提示

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 前端 | Vue 3 + Vite + TypeScript + Naive UI + pnpm |
| 后端 | Rust + axum + SQLx + SQLite |
| 工具 | Biome / oxlint / Docker Compose / GitHub Actions |
| 部署 | 单 Docker 镜像，后端直接托管前端静态资源 |

## 快速开始

要求 Docker Engine 24+ 和 Docker Compose v2。

```bash
git clone https://github.com/ydfk/service-compass.git
cd service-compass
docker compose up -d --build
```

打开：

```text
http://服务器地址:3010
```

默认账号和密码均为 `admin`。首次登录后请立即在管理端修改账号密码。

数据、SQLite 数据库和加密密钥默认保存在 `./data`，升级或迁移前请备份该目录。

## 使用已发布镜像

```bash
curl -LO https://github.com/ydfk/service-compass/releases/latest/download/docker-compose.prod.yml
docker compose -f docker-compose.prod.yml up -d
```

默认端口是 `3010`。如需修改：

```bash
SERVICECOMPASS_PORT=8080 docker compose -f docker-compose.prod.yml up -d
```

## Docker 发现

Docker 发现只生成候选服务，不会自动添加服务。你需要在管理端选择候选容器并保存，系统才会创建服务、关联容器，并按需创建 Docker 监控。

本机 Docker 发现需要在 Compose 文件中启用 Docker Socket 挂载：

```yaml
- /var/run/docker.sock:/var/run/docker.sock:ro
```

> [!WARNING]
> Docker Socket 权限很高。即使只读挂载，也可能暴露容器、镜像、网络、挂载路径和环境信息。仅建议在可信内网主机启用，不要把未加密的 Docker API 暴露到公网。

## 通知配置

支持：

- Bark
- 通用 Webhook
- Synology Chat Incoming Webhook / Chatbot

通知通道负责“发到哪里”和“对哪些服务生效”；每个监控可单独设置是否通知、通知通道、离线/恢复/警告和冷却时间。

Synology Chat 建议优先使用 Incoming Webhook。若使用 Chatbot 模式，需要填写频道 ID 或用户 ID。

## 本地开发

需要 Rust stable、Node.js 22+ 和 pnpm 11+。

```powershell
pnpm -C frontend install
.\scripts\dev.ps1
```

常用检查：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C frontend lint
pnpm -C frontend build
docker compose build
```

也可以使用脚本：

```powershell
.\scripts\check.ps1
.\scripts\docker-build.ps1
.\scripts\docker-test.ps1
```

`docker-test.ps1` 会构建镜像、启动 `docker compose`、等待 `/api/health` 通过，然后输出本地访问地址。常用参数：

```powershell
.\scripts\docker-test.ps1 -NoCache
.\scripts\docker-test.ps1 -StopAfterTest
```

## 发布

推送 `v1.2.3` 格式的标签后，GitHub Actions 会构建 Docker 镜像并创建 GitHub Release。

发布镜像会把 Git tag 注入为应用版本号；例如 `v1.2.3` 对应管理端显示 `v1.2.3`，Docker 镜像会同时生成 semver tag。

仓库需要配置：

- `DOCKERHUB_USERNAME`
- `DOCKERHUB_TOKEN`

## 常见注意事项

- ServiceCompass 聚焦 MVP 能力，不追求替代完整监控平台。
- Docker 发现不会自动导入容器，避免把临时容器或基础设施误加为服务。
- 通知 Token、Docker TLS 私钥等配置会加密保存；前端不会拿到明文 secret。
- 日志会尽量记录关键节点，但应避免把真实 Token、密码粘贴到公开 issue。

## License

MIT
