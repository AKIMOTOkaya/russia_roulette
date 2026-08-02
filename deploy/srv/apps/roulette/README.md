# Roulette 服务器应用

该目录镜像服务器 `~/srv/apps/roulette`，当前包含一个 `roulette-backend` 服务。

## 部署运行文件

将为 Linux 目标构建的可执行文件放到：

```text
services/roulette-backend/runtime/roulette-backend
```

并保证文件具有执行权限。Rust 应用源码已可运行本地 Web MVP，但本模板尚未生成 Linux 服务端文件；当前实现也只有进程内单局，不是完成的公网多人服务。

## 网络

- 服务只加入外部网络 `srv_edge`。
- Compose 仅 `expose` 内部端口 `8080`，不发布宿主机端口。
- Caddy 上游地址为 `roulette-backend:8080`。
- 当前不加入数据库使用的 `srv_data` 网络。

## 检查与启动

在服务器影子目录或服务器对应目录中执行：

```bash
docker compose config
docker compose up -d
docker compose ps
```

在 Linux 可执行文件、Caddy 路由和公网访问安全边界准备好之前，只执行 `docker compose config`，不要启动服务。
