# 2026-09-19：中央服务器应用容器配置套件落地与生产部署验证

## 用户目标

- 深入 `~/Documents/srv` 学习生产级应用服务目录的容器与部署规范；
- 完成系列容器配置文件（`docker-compose.yml`, `Dockerfile`, `.dockerignore`, `.env` 等），为生产部署做好准备；
- 预先生成静态管理员 Token 保存在配置中，防止容器每次重启重新生成；
- 适配 Caddy 边缘反向代理域名 `rrt.akiai.asia`；
- 在本项目目录与本机影子目录各安装一份完整的生产部署物料；
- 编译生成 Linux x86_64 二进制，推送到远程中央服务器（`106.54.211.86`）启动服务并完成全套在线验证。

## 调研与规范学习

在 `~/Documents/srv` 中研读了 `biligo`、`torus`、`mini-tools`、`openlist` 以及 `ops/scripts/push.sh`：
1. **轻量生产部署哲学**：远程服务器无需安装庞大编译工具链，运行时通过精简镜像（`debian:12-slim` 或 `distroless`）挂载只读生产二进制运行；
2. **两级环境变量设计**：根目录 `.env` 仅由 `sync-app-compose-env.sh` 写入容器外部端口，`services/<service>/.env` 保存完整运行时配置；
3. **网络安全隔离**：容器仅通过 `expose` 暴露端口并加入 `srv_edge` 外部共享网络，不直接向宿主机公网暴露端口；由 Caddy 自动申请 TLS 证书并向内部容器名称反代。

## 设计与实施细节

### 1. 静态管理员 Token 生成与持久化
- 生成 32 位静态高熵十六进制 Token（写入生产环境私密配置，仓库以 `.env.example` 占位）；
- 写入 `deploy/srv/apps/roulette/services/roulette-backend/.env` 中的 `ROULETTE_ADMIN_TOKEN`，容器重启时不发生漂移。

### 2. 容器配置套件落盘
- **`docker-compose.yml`**：定义 `roulette-backend` 服务，使用 `debian:12-slim` 基础镜像，注入环境并挂载 `runtime` 目录至 `/app/runtime:ro`，接入 `srv_edge` 外部网络；
- **`services/roulette-backend/Dockerfile`** 与 **`.dockerignore`**：提供服务级标准 Docker 构建定义；
- **根目录 `Dockerfile`** 与 **`.dockerignore`**：提供基于 `rust:1.85-slim-bookworm` 的多阶段（Multi-stage）完整构建支持；
- **`deploy/srv/apps/roulette/README.md`**：编写完备的应用维护、反代配置与运维命令说明。

### 3. 构建与打包自动化 (`scripts/build-srv.sh`)
- 编写一键构建脚本，支持本地 Docker 或自动化通过构建通道编译 Linux x86_64 生产可执行文件（`roulette-server`）；
- 产物写入 `runtime/roulette-backend`（赋权 `chmod +x`），生成 `build-info.md` 记录构建参数、时间戳与 Git 提交；
- 自动同步至本机服务器影子目录 `/Users/akimotokaya/Documents/srv/apps/roulette/`。

### 4. 网关与推送部署
- 在 `gateway/caddy/Caddyfile` 配置反代：
  ```caddyfile
  rrt.akiai.asia {
      reverse_proxy roulette-backend:8080
  }
  ```
- 通过 `ops/scripts/push.sh caddy --restart` 更新网关并热重载；
- 通过 `ops/scripts/push.sh roulette --restart` 推送容器配置与二进制，平滑创建并启动容器 `roulette-roulette-backend-1`。

## 验证与在线测试

1. **容器运行状态与日志**：
   - 容器 `roulette-roulette-backend-1` 状态：`Up, 8080/tcp`；
   - 容器日志确认启动为公网中央服务器模式，MCP 工具及 Streamable 端点正常挂载，管理员凭据已加载。
2. **公网 HTTPS 端点实测 (`https://rrt.akiai.asia`)**：
   - `GET /`：HTTP 200，成功返回 Web 客户端 HTML；
   - `GET /api/health`：`{"status":"ok"}`；
   - `GET /api/server/settings?tab_id=probe`：返回 `{"server_mode":"public","founder_authenticated":false,"lan_exposed":true}`；
   - `POST /api/rooms`：成功在公网中央服务器创建游戏房间（ID: `XSCUU`）；
   - `GET /api/mcp/tools`：成功输出 9 个裁判特权工具及玩家工具 JSON Schema。
3. **工作区代码规范**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `cargo test --workspace`：38 项测试全部通过。
