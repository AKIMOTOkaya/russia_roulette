# 仓库结构

## 当前目录树

```text
RussianRoulette/
├── AGENTS.md
├── Cargo.toml
├── rust-toolchain.toml
├── crates/
│   ├── roulette-domain/
│   ├── roulette-core/
│   └── roulette-host/
├── apps/
│   └── roulette-backend/
├── clients/
│   └── web/
├── protocol/
├── docs/
│   ├── rules/
│   ├── architecture/
│   ├── development/
│   ├── project/
│   └── development-log/
├── scripts/
├── tests/
└── deploy/
```

## 顶级目录职责

| **目录** | **放置内容** | **不放置内容** |
|----------|--------------|----------------|
| `crates/` | 可复用 Rust 库。 | `main`、部署配置、Web 页面。 |
| `apps/` | Rust 可执行程序和组合入口。 | 重复实现的领域规则。 |
| `clients/` | Web、桌面等玩家客户端。 | 完整 `GameState` 或权威裁定。 |
| `protocol/` | 跨语言 schema、兼容说明、生成配置。 | 手工复制的多语言 DTO。 |
| `docs/` | 规则、架构、项目说明与开发日志。 | 构建产物和运行数据。 |
| `scripts/` | 可重复执行的生成、检查、迁移和运维脚本。 | 无说明的一次性个人命令。 |
| `tests/` | 跨模块集成、兼容、恢复和端到端测试。 | 单个 crate 的普通单元测试。 |
| `deploy/` | 容器、反向代理和环境部署配置。 | 密钥或环境专属凭证。 |

## 扩展规则

- 新建 crate 前必须能写清单一职责、允许依赖和至少一个实际使用方。
- 新语言按产品或 SDK 边界建立目录，不在根目录散放源文件。
- 数据库迁移目录在选定存储方案时建立；不会为了占位提前创建。
- 生成代码必须有稳定来源和生成命令，原则上不手工修改。
- 根目录只保留全仓库入口、统一配置和少量治理文件。
- `AGENTS.md` 记录对后续开发会话生效的仓库级维护与交付规则。
