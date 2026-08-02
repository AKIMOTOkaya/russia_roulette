# 开发日志：服务器运行容器骨架

- 日期：2026-08-03
- 会话：001
- 状态：完成
- Git：与本日志同一提交；提交信息 `chore: add server container skeleton`

## 用户目标

记录服务器影子目录、独立应用 Compose、服务级环境变量、共享数据库和 Caddy 网络约定；扫描现有应用格式，并只在本项目目录中增加服务器运行容器。

## 范围与假设

- 本机 `/Users/akimotokaya/Documents/srv` 对应服务器 `~/srv`。
- 新应用目录使用现有复数路径 `apps/roulette`。
- 当前后端尚无可运行的 Linux 二进制，也尚未接入数据库。
- 公网域名和 Caddy 路由尚未决定。

## 关键决定

- Compose 服务名和容器网络 DNS 名统一为 `roulette-backend`。
- 使用预载二进制 + Distroless CC 镜像的运行模式，匹配现有服务器应用结构并支持 Rust GNU 目标二进制。
- 根 `.env` 只保存 `ROULETTE_HTTP_PORT`；服务 `.env` 保存完整非敏感运行变量。
- 服务只加入 `srv_edge`，不发布宿主机端口，不接入 `srv_data`。
- 不修改 Caddy、共享数据库、全局脚本或其他应用文件；域名和数据库方案确定后分别处理。
- 项目仓库保留部署模板，再复制到服务器影子目录，避免影子目录成为唯一配置来源。

## 修改内容

- 新增 `docs/project/deployment.md`，记录影子目录、环境变量分层和共享网络边界。
- 更新项目概览、仓库结构、模块说明和开发流程。
- 新增 `deploy/srv/apps/roulette` Compose 模板、应用/服务环境变量与运行目录。
- 在服务器影子目录中仅创建 `apps/roulette` 及相同文件。

## 验证

- 只读扫描多个单服务、多服务、Caddy 和数据库 Compose 目录。
- 读取现有 `.env` 时仅检查变量名，没有复制或输出既有值。
- 验证项目模板与服务器影子目录文件一致。
- 使用 Compose 配置解析检查服务、端口和外部网络声明。
- Rust 工具链仍不可用，本次未执行 Cargo 检查。

## 遗留事项

- 实现 `roulette-backend` HTTP/WSS 服务并构建 Linux 可执行文件。
- 确定公网域名后，由运维修改 Caddy 配置。
- 确定 PostgreSQL 角色、库名、凭证注入和迁移方案后，再接入 `srv_data`。
