# 俄罗斯轮盘（暂定名）

多人、多端、权威规则驱动的回合制轻游戏。项目最初通过 QQ 群主持 Bot 游玩，当前优先用本地 Web MVP 验证玩法和核心抽象，再扩展公网多人版。

## 仓库结构

```text
RussianRoulette/
├── AGENTS.md              # 仓库级开发与交付约束
├── Cargo.toml             # Rust 工作区入口
├── rust-toolchain.toml    # Rust 工具链约定
├── crates/                # 可复用 Rust 库，不包含进程启动逻辑
│   ├── roulette-domain/   # 领域 ID、命令、事件、状态与视图
│   ├── roulette-core/     # 确定性规则状态机
│   └── roulette-host/     # 多房间生命周期、权限、Tab 映射与 Bot 调度
├── apps/                  # Rust 可执行程序（backend、node、CLI 等）
│   └── roulette-backend/  # 当前本地 HTTP 服务与未来公网后端组合入口
├── clients/               # Web、桌面等客户端，可使用不同语言
│   └── web/               # 当前优先的 Web 客户端
├── protocol/              # 跨语言网络协议和代码生成配置
├── docs/                  # 规则、架构和开发路线
├── scripts/               # 开发、生成、检查和运维辅助脚本
├── tests/                 # 跨模块集成、协议兼容和端到端测试
└── deploy/                # 容器、反向代理和环境部署配置
    └── srv/apps/roulette/ # 服务器 ~/srv/apps/roulette 的可追踪模板
```

根目录不放普通业务源码。Rust 库放入 `crates/`，Rust 可执行程序放入 `apps/`，其他语言按产品形态放入 `clients/`、未来的 `sdk/` 或相应独立目录。

## 当前状态

- 已实现 `roulette-domain`、确定性 `roulette-core`、本地 `roulette-host` 与 Axum `roulette-backend` 的 MVP 纵向切片。
- 已实现本地多房间大厅。每个浏览器标签页是一名独立真人玩家，任何人可创建或通过五位房间号加入房间，房主可设置密码、管理 Bot、转让房主、开局和解散房间。
- 已实现 3～6 名真人/Bot 混合对局、地图、紧凑操作区、移动、射击、等待、自裁和胜负结算；真人退出进行中对局时由 Bot 接管其席位。
- 后端启动时支持注入创始人临时密码；本机页面验证后可以实时开启或关闭局域网访问。
- 对局使用从上到下的单列顺序记录流，分别表达玩家与 Bot 行动、独立事件、回合推进和系统通知，为后续事件系统保留扩展边界。
- 已整理正式规则、本地 MVP 规则与抽象设计；Web 公网版开发路径暂时保留为后续支线。
- 已建立 `roulette-backend` 服务器容器骨架，当前只加入 Caddy 使用的 `srv_edge` 网络。
- 本地 MVP 使用 Axum 和原生 Web 技术，不接数据库；公网客户端框架、数据库访问库和公开协议仍未冻结。

## 运行本地 MVP

```bash
cargo run -p roulette-backend
```

启动时终端会显示临时创始人密码。也可以显式注入：

```bash
cargo run -p roulette-backend -- --founder-password local-only
```

打开 <http://127.0.0.1:8787>。房间和对局只保存在内存中，停止程序后不会保留。服务监听所有本机接口，但默认拒绝非回环地址；需由已认证的创始人在页面中开启局域网访问。

## 文档入口

- [核心玩法规则](docs/rules/game-rules.md)
- [本地 Web MVP 规则](docs/rules/local-web-mvp-rules.md)
- [本地 Web MVP 架构与抽象](docs/architecture/local-web-mvp-design.md)
- [本地 Web MVP 开发与运行](docs/development/local-web-mvp.md)
- [总体架构与开发说明](docs/architecture/俄罗斯轮盘_架构与开发说明.md)
- [Web 公网版开发路径](docs/development/web-server-development-path.md)
- [项目说明文档组](docs/project/README.md)
- [开发日志](docs/development-log/README.md)
- [文档目录与维护规则](docs/README.md)

## Rust 开发约定

安装 Rust 后，在仓库根目录运行：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

新增 Rust 模块时遵循以下边界：

- 领域类型进入 `crates/roulette-domain`。
- 确定性规则进入 `crates/roulette-core`。
- 房间、重连、Bot 调度和持久化端口进入 `crates/roulette-host`。
- HTTP/WebSocket、数据库和进程启动逻辑进入 `apps/` 或专用适配器 crate。
- 客户端不得依赖或复制权威规则实现。

详细结构、模块边界、命名约定和开发流程以 [`docs/project/`](docs/project/README.md) 为准。每次开发会话都必须同步项目说明、写一篇开发日志，并形成一次 Git 提交。
