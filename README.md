# ServiceCompass

轻量级、自托管的服务导航与状态监控面板，适用于 Homelab、NAS 和个人服务器。

ServiceCompass 以服务为核心，可按需关联 Docker 容器并启用监控。它专注于服务首页和基础可用性监控，不以替代 Uptime Kuma 为目标。

## 功能

- 分组服务首页，支持大小卡片、排序和登录后直接编辑
- HTTP/HTTPS、响应关键字、DNS、证书和 Docker 状态监控
- 分别展示 HTTP 与 Docker 可用率及近期状态
- Docker 容器发现、图标匹配和手动上传图标
- Bark、Webhook 和 Synology Chat 通知
- 管理员登录、系统日志和敏感配置加密存储
- Rust + Axum + SQLite 后端，Vue 3 前端，单 Docker 镜像部署

## 快速开始

要求 Docker Engine 24+ 和 Docker Compose v2。

```bash
git clone https://github.com/ydfk/service-compass.git
cd service-compass
docker compose up -d --build
```

打开 `http://服务器地址:3010`。

默认账号和密码均为 `admin`，首次登录后请立即在设置中修改。数据和加密密钥保存在 `./data`，升级或迁移前请备份该目录。

### 使用已发布镜像

```bash
curl -LO https://github.com/ydfk/service-compass/releases/latest/download/docker-compose.prod.yml
docker compose -f docker-compose.prod.yml up -d
```

默认镜像为 `ydfk/service-compass:latest`。可通过 `SERVICECOMPASS_PORT` 修改端口：

```bash
SERVICECOMPASS_PORT=8080 docker compose -f docker-compose.prod.yml up -d
```

## Docker 发现

Docker 发现只生成候选服务，不会自动添加。选中容器并保存服务后，系统才会建立关联和 Docker 状态监控。

本机发现需要取消 Compose 文件中的 Docker Socket 挂载注释：

```yaml
- /var/run/docker.sock:/var/run/docker.sock:ro
```

Docker Socket 具有较高权限，即使只读挂载也会暴露容器、网络、挂载和环境信息。仅应在可信主机中启用，不要将未加密的 Docker API 暴露到公网。

## 通知

支持 Bark、通用 Webhook 和 Synology Chat。通知通道可先发送测试，再按监控配置离线、恢复、警告和冷却时间。

监控密码、Docker TLS 私钥和通知配置会加密保存。日志不会记录密码、Token 或 Authorization Header。

## 本地开发

需要 Rust stable、Node.js 22+ 和 pnpm 11+。

```powershell
pnpm -C frontend install
.\scripts\dev.ps1
```

运行完整检查：

```powershell
.\scripts\check.ps1
.\scripts\docker-build.ps1
```

## 发布

推送 `v1.2.3` 格式的标签后，GitHub Actions 会发布 Linux amd64 镜像并创建 GitHub Release。仓库需要配置 `DOCKERHUB_USERNAME` 和 `DOCKERHUB_TOKEN`。

## License

MIT
