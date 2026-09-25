# Roulette (俄罗斯轮盘) 公网服务器应用

该目录镜像服务器 `~/srv/apps/roulette`，提供俄罗斯轮盘公网中央服务器对局与 MCP 接口服务。

## 服务构成

- `roulette-backend`: 基于 `gcr.io/distroless/cc-debian12:nonroot` 最小化运行镜像（或通过 `services/roulette-backend/Dockerfile` 构建），挂载并运行经过静态优化的 Linux x86_64 二进制。

## 网络与访问

- 服务加入共享外部网络 `srv_edge`。
- Compose 仅 `expose` 内部端口 `${ROULETTE_HTTP_PORT:-8080}`，不向宿主机发布直接端口（防暴力扫描与未授权绕过）。
- 边缘网关 Caddy 反代域名 `rrt.akiai.asia` 到 `roulette-backend:8080`：
  ```caddyfile
  rrt.akiai.asia {
      reverse_proxy roulette-backend:8080
  }
  ```

## 运行环境变量

配置文件位于 `./services/roulette-backend/.env`（仓库中由 `.env.example` 提供模板，真实 `.env` 纳入 `.gitignore` 保护，严禁提交到代码仓库）：
- 可通过模板创建配置：`cp services/roulette-backend/.env.example services/roulette-backend/.env`
- `ROULETTE_ENV=production`：生产运行环境；
- `ROULETTE_SERVER_MODE=public`：公网中央服务器模式（自动放行反向代理流量并压制本地回环检查）；
- `ROULETTE_HTTP_HOST=0.0.0.0`
- `ROULETTE_HTTP_PORT=8080`
- `ROULETTE_ADMIN_TOKEN`：静态管理员与裁判 Token（避免重启变化，持久化保存在本地私密配置中）；
- `ROULETTE_PUBLIC_BASE_URL=https://rrt.akiai.asia`：公网外部访问地址；
- `ROULETTE_RUNTIME_DIR=/app/runtime`：运行时目录。

## 构建与部署流程

1. **构建生产 Linux 二进制**：
   在源码仓库根目录下执行构建脚本：
   ```bash
   ./scripts/build-srv.sh
   ```
   该脚本会自动利用 Docker 或交叉编译器编译 Linux x86_64 生产版本，放置到 `runtime/roulette-backend`，并同步安装到本机影子目录 `~/Documents/srv/apps/roulette/`。

2. **推送到中央服务器并重启**：
   在影子仓库根目录执行：
   ```bash
   ./ops/scripts/push.sh roulette --restart
   ```

3. **运维与诊断命令**：
   ```bash
   # 查看运行容器状态
   ssh aki@106.54.211.86 "docker compose -f ~/srv/apps/roulette/docker-compose.yml ps"

   # 查看实时服务日志
   ssh aki@106.54.211.86 "docker logs -f roulette-roulette-backend-1"

   # 校验健康检查与设置接口
   curl -s https://rrt.akiai.asia/api/health
   curl -s https://rrt.akiai.asia/api/server/settings?tab_id=probe
   ```
