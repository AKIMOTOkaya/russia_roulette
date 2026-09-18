# 部署约定

## 服务器影子目录

本机 `/Users/akimotokaya/Documents/srv` 是服务器 `~/srv` 的影子目录。该目录是独立仓库，并且可能包含其他应用的未提交修改。处理本项目部署时只能创建或修改：

```text
/Users/akimotokaya/Documents/srv/apps/roulette/
```

不得顺手格式化、生成、提交或修改其他应用、`gateway/`、`infra/`、`ops/` 和影子仓库根文件。

本项目仓库中的可追踪模板位于：

```text
deploy/srv/apps/roulette/
```

## 应用目录约定

服务器 `apps/` 下每个一级目录是一套独立 Compose 应用：

```text
apps/<app>/
├── docker-compose.yml
├── .env
└── services/
    └── <service>/
        ├── .env
        └── runtime/
```

- 应用根 `.env` 只保存 Compose 插值所需的端口变量。
- `services/<service>/.env` 保存容器预载的完整运行环境变量。
- 服务运行文件放入自己的 `runtime/`，以只读卷挂载到容器。
- 不在 Compose 文件中写入密码、Token 或数据库凭证。

根 `.env` 的端口区块与服务器工具 `ops/scripts/sync-app-compose-env.sh` 的输出格式一致，但本项目创建期间不运行该全局脚本，避免改动其他应用文件。

## 共享网络

| **网络** | **管理位置** | **用途** | **本项目当前状态** |
|----------|--------------|----------|--------------------|
| `srv_edge` | `gateway/caddy` | Caddy 与所有公网应用容器之间的内部反向代理网络。 | 已加入。 |
| `srv_data` | `infra/db` | PostgreSQL 与使用共享数据库的应用容器网络。 | 暂不加入。 |

`roulette-backend` 只使用 `expose` 声明容器端口，不发布宿主机公网端口。Caddy 通过内部网络反向代理：
```caddyfile
rrt.akiai.asia {
    reverse_proxy roulette-backend:8080
}
```
当前已正式配置域名 `rrt.akiai.asia` 接入 Caddy 并完成 HTTPS 证书签发与在线验证。

数据库接入必须等数据库角色、库名、凭证注入和迁移策略确定后再实施。届时才为服务增加 `srv_data` 网络与数据库环境变量。

## 当前容器与部署产物

| **Compose 服务** | **基础镜像** | **入口** | **内部端口** | **网络** | **公网访问** |
|------------------|--------------|----------|--------------|----------|--------------|
| `roulette-backend` | `debian:12-slim` | `/app/runtime/roulette-backend` | `8080` | `srv_edge` | `https://rrt.akiai.asia` |

- **二进制构建**：在仓库根目录执行 `./scripts/build-srv.sh`，构建 Linux x86_64 生产版本，并自动同步安装到 `deploy/srv/apps/roulette/services/roulette-backend/runtime/` 及本机影子目录 `/Users/akimotokaya/Documents/srv/apps/roulette/`。
- **推送到服务器**：在影子目录根目录执行 `./ops/scripts/push.sh roulette --restart`。
- **凭据管理**：静态管理员 Token（`ROULETTE_ADMIN_TOKEN`）持久化保存在 `services/roulette-backend/.env` 中，避免容器重启导致 Token 漂移。

