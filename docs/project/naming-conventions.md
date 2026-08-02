# 命名约定

## 统一项目命名空间

代码模块统一使用 `roulette` 前缀。包名不使用过于泛化的 `game-*`，也不使用难以搜索的 `rr-*` 缩写。

| **对象** | **格式** | **示例** |
|----------|----------|----------|
| Cargo package | `kebab-case` | `roulette-domain` |
| Rust crate / module | `snake_case` | `roulette_domain`、`match_state` |
| Rust 类型 / trait | `PascalCase` | `PlayerCommand`、`GameEngine` |
| Rust 函数 / 字段 | `snake_case` | `apply_command`、`expected_revision` |
| Rust 常量 | `SCREAMING_SNAKE_CASE` | `MAX_ROOM_PLAYERS` |
| 可执行程序 | `kebab-case` | `roulette-backend` |
| 顶级目录 | 简短英文复数名词 | `crates`、`clients`、`scripts` |
| 协议字段 | `snake_case` | `idempotency_key` |
| 文档 | 英文 `kebab-case` 或稳定中文标题 | `modules.md`、`俄罗斯轮盘_架构与开发说明.md` |

## 语义约定

- `match` 表示一局权威游戏；`room` 表示玩家加入和准备的会话容器。
- `command` 表示尚待校验的意图；`event` 表示已经发生的领域事实。
- `view` 表示按观察者身份投影后的可见状态；不得用 `state` 指代客户端视图。
- `host` 表示通用会话编排层；`backend` 表示公网服务进程；`node` 表示本地或 LAN 承载进程。
- `adapter` 用于协议、存储或平台边界；普通领域服务不使用该后缀。

## 禁止的模糊命名

没有明确边界时不创建 `common`、`shared`、`utils`、`misc`、`manager` 或 `helpers` 模块。可复用代码应以其业务职责命名，并放入依赖方向正确的模块。
