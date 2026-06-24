# ServiceCompass / 服务罗盘

ServiceCompass 是面向 Homelab、NAS 和个人服务器的轻量服务首页。服务是唯一必需实体；创建服务时可以关联 Docker 容器，并可同时建立完整的 HTTP/HTTPS 或响应关键字监控。

它不是 Uptime Kuma 的完整替代品。MVP 包含分组化服务首页、内外网入口、HTTP/关键字/DNS/证书/Docker 状态监控、Docker 候选发现、通知和系统日志。

## Docker 安装

要求 Docker Engine 24+ 与 Docker Compose v2：

```bash
git clone <your-repository-url> service-compass
cd service-compass
docker compose up -d --build
```

打开 `http://服务器地址:3010`。默认管理员用户名和密码都是 `admin`，首次登录后请在“设置 → 常规与账号”立即修改。数据、上传图标和本地加密密钥保存在 `./data`，升级前应备份整个目录。

单个 Docker 镜像同时包含 Rust 后端和 Vue 前端，前端 `dist` 由后端直接托管。

也可以使用 Docker Hub 中的已发布镜像：

```bash
curl -LO https://github.com/ydfk/service-compass/releases/latest/download/docker-compose.prod.yml
docker compose -f docker-compose.prod.yml up -d
```

生产 Compose 默认拉取 `ydfk/service-compass:latest`。可以通过 `SERVICECOMPASS_PORT` 修改宿主机端口，例如 `SERVICECOMPASS_PORT=8080 docker compose -f docker-compose.prod.yml up -d`。

## 配置

| 环境变量 | 镜像默认值 | 说明 |
|---|---|---|
| `DATABASE_URL` | `sqlite:/data/service-compass.db` | SQLite 数据库 URL |
| `SERVICECOMPASS_BIND` | `0.0.0.0:3000` | 监听地址 |
| `SERVICECOMPASS_SECRET_KEY` | 未设置 | 可选的敏感字段加密主密钥 |
| `SERVICECOMPASS_SECRET_FILE` | `/data/secret.key` | 自动生成并保存加密密钥的路径 |
| `SERVICECOMPASS_STATIC_DIR` | `/app/frontend/dist` | 前端静态文件目录 |
| `RUST_LOG` | `info` | 日志过滤级别 |

Compose 无需设置环境变量即可运行。Docker TLS 私钥和监控密码以加密 secret 保存，首次启动生成的 `/data/secret.key` 必须随数据目录一起备份。通知通道配置同样加密落盘，但登录管理端后可直接读取和编辑。更换加密密钥后旧密文无法解密。

## 服务、Docker 与监控

在“服务”页面点击“添加服务”：

1. 填写名称和至少一个访问地址；外网地址在前，内网地址可选，分组也可不选。
2. 需要 Docker 关联时打开“关联 Docker”，选择端点并扫描，再人工选择候选容器。
3. 需要 HTTP 监控时打开“服务监控”，可配置 HTTP/HTTPS、响应关键字、方法、状态码、超时、重试、TLS 和 Basic Auth。

一旦关联 Docker 容器，系统会强制创建独立的 Docker 状态监控，读取容器运行状态和 Docker Health Check。HTTP 与 Docker 会作为两条状态轨道分别展示。

首页支持小卡片和监控详情卡片切换。详情卡片展示过去 24 小时最近 30 次检查的状态条与可用率；登录后可以在首页直接编辑服务和调整同组卡片顺序。

Docker 扫描只会生成候选，不会自动添加服务。只有在服务弹框中选中候选并保存，关联才会写入服务。

Docker 端点管理位于“设置 → Docker”。启用本机发现时，取消 `docker-compose.yml` 中 Socket 挂载的注释：

```yaml
- /var/run/docker.sock:/var/run/docker.sock:ro
```

远程端点使用 `tcp://host:2376`，推荐配置 TLS CA、客户端证书和私钥。非 TLS 连接只允许私网地址。

### Docker Socket 风险

Docker Socket 是高权限入口。即使使用 `:ro` 挂载，访问它的进程仍可读取容器、网络、挂载和环境信息；只读挂载不等于低权限。

- 仅在可信宿主机和可信内网启用。
- 不需要发现时不要挂载 Socket。
- 不要把无 TLS 的 Docker API 或 ServiceCompass 管理端直接暴露到公网。
- 对外访问优先使用 VPN；反向代理时再加 HTTPS 和访问控制。

## 通知配置

支持 Bark、通用 Webhook 和 Synology Chat。先在“通知”中新建通道并发送测试，再按监控或全部监控建立规则。规则可分别控制离线、恢复和警告，并带冷却时间。

- Bark：填写 Server 和 Device Key。
- Webhook：填写 URL、方法和可选 Headers JSON。
- Synology Chat：填写 DSM Chat 地址和 Token；`chatbot` 模式需选择频道或用户目标，`incoming` 模式使用 Incoming Webhook Token。请求使用 Synology Chat 的 `payload` 表单格式，HTTP 200 但 `success=false` 仍会显示为失败。

通知配置加密落盘，登录管理端后可直接回看和修改；系统日志不会记录 Token、Authorization Header 或密码。

## Windows 本地开发

要求 Rust stable、Node.js 22+ 与 pnpm 11+。根目录不是 pnpm workspace，pnpm 只在 `frontend` 内使用。

```powershell
pnpm -C frontend install
.\scripts\dev.ps1
```

常用验收命令已收纳为 PowerShell 脚本：

```powershell
.\scripts\check.ps1
.\scripts\docker-build.ps1
```

也可分别运行：

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm -C frontend install
pnpm -C frontend build
docker compose build
```

Vite 开发服务器会把 `/api` 代理到 `127.0.0.1:3000`。系统日志在“设置 → 系统日志”按时间倒序显示，当前进程最多保留 1000 条。

## 自动发布

推送 `v1.2.3` 形式的版本标签后，GitHub Actions 会构建并发布 Linux amd64 镜像到 Docker Hub，同时创建包含 `docker-compose.prod.yml` 的 GitHub Release。正式版本会更新 `latest`，`v1.2.3-rc.1` 等预发布标签会创建 Prerelease，但不会更新 `latest`。

首次发布前，需要在 GitHub 仓库的 Actions secrets 中配置：

- `DOCKERHUB_USERNAME`：Docker Hub 用户名。
- `DOCKERHUB_TOKEN`：拥有镜像推送权限的 Access Token。

## License

MIT
