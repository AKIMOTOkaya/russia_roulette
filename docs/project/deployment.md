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

`roulette-backend` 只使用 `expose` 声明容器端口，不发布宿主机公网端口。Caddy 未来通过 `roulette-backend:8080` 反向代理；域名确定后再由运维单独修改 `gateway/caddy/Caddyfile`。

数据库接入必须等数据库角色、库名、凭证注入和迁移策略确定后再实施。届时才为服务增加 `srv_data` 网络与数据库环境变量。

## 当前容器

| **Compose 服务** | **镜像** | **入口** | **内部端口** | **网络** |
|------------------|----------|----------|--------------|----------|
| `roulette-backend` | `gcr.io/distroless/cc-debian12:nonroot` | `/app/runtime/roulette-backend` | `8080` | `srv_edge` |

当前源码已能作为本地 Web MVP 运行，但服务器模板仍只是部署骨架：尚未产出 Linux 可执行文件，也未配置 Caddy 路由。将本地单局暴露到公网还缺少身份、访问隔离和安全评审，因此不能把当前二进制直接视为公网多人服务。
