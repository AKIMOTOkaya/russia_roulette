# 仓库开发规则

本文件适用于整个仓库。

## 开始开发前

- 阅读根目录 `README.md` 和 `docs/project/README.md`。
- 涉及玩法时阅读 `docs/rules/`；涉及架构或模块时阅读 `docs/architecture/` 与 `docs/project/modules.md`。
- 保持服务器权威和单向依赖边界，不在客户端复制规则裁定。

## 命名与目录

- Rust 包和可执行程序统一使用 `roulette-*` 前缀。
- Cargo package 使用 `kebab-case`，Rust crate/module 使用 `snake_case`。
- 不创建职责不清的 `common`、`shared`、`utils`、`misc`、`manager` 或 `helpers` 模块。
- Rust 库放 `crates/`，Rust 应用放 `apps/`，客户端放 `clients/`，跨语言协议放 `protocol/`。

## 每次开发会话的交付要求

只要本次会话修改了仓库，就必须：

1. 同步更新 `docs/project/` 中受影响的项目说明。
2. 在 `docs/development-log/` 新建一篇按日期和序号命名的会话日志。
3. 执行与改动风险相称的格式检查、静态检查和测试；不能执行时在日志中说明原因。
4. 检查 Git diff，不提交密钥、构建产物或无关修改。
5. 将本次会话修改整理为一次 Git 提交，并向用户报告提交哈希。

只读解释或诊断会话不创建空提交；若形成了新的项目决定，则更新文档和日志后提交。

## 文档来源

- Markdown 是持续维护源稿。
- DOCX 是阶段性评审快照，只在里程碑或明确要求时同步导出。
- 旧日志不重写；决定变化时在新日志说明并更新当前项目说明。

## Rust 验证

工具链可用时，默认运行：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```
