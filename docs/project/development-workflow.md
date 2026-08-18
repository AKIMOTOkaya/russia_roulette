# 开发流程

## 每次开发会话

每次与开发相关的会话按以下顺序收口：

1. 明确本次目标、范围和已知假设。
2. 修改代码、脚本、配置或文档。
3. 同步更新 `docs/project/` 中受影响的项目说明。
4. 在 `docs/development-log/` 新建一篇会话日志。
5. 执行与风险相称的格式、静态检查、测试或结构验证。
6. 检查 Git diff，确认不包含无关文件或敏感信息。
7. 将本次会话修改合并为一次 Git 提交，并在交付说明中报告提交哈希。

如果会话只做解释或只读诊断，没有修改仓库，则不要求创建日志或空提交。若用户明确要求每个开发会话都提交，则以文档日志作为最小可提交内容。

## `AGENTS.md` 的维护

- 根 `AGENTS.md` 是当前仓库的开发协作约定，不是不可修改的产品规格。
- 用户最新明确决定可以替换其中的当前实现假设；发生变化时，应在同一会话更新 `AGENTS.md`、本目录受影响说明和新开发日志。
- 服务器权威、确定性 Core 和单向依赖等稳定不变量，与原生前端、Tab 身份、本地密码等阶段性基线应分开表述，避免临时实现被误当成永久约束。
- `AGENTS.md` 保持面向执行，不堆积历史过程；历史原因和已废弃决定留在开发日志中。

## 文档同步

| **发生变化** | **至少更新** |
|--------------|--------------|
| 产品范围或当前路线 | `overview.md`、相关开发路径、会话日志。 |
| 顶级目录 | `repository-structure.md`、根 `README.md`、会话日志。 |
| 模块职责或依赖 | `modules.md`、总体架构、会话日志。 |
| 命名规则 | `naming-conventions.md`、受影响代码、会话日志。 |
| 开发或提交规则 | `development-workflow.md`、日志模板。 |
| 仓库级开发约束 | 根 `AGENTS.md`、`development-workflow.md`、会话日志。 |
| 部署目录、容器或网络 | `deployment.md`、模块说明、会话日志。 |
| 权威玩法 | `docs/rules/`、相关 Core 测试、会话日志。 |

## Git 提交

提交信息采用简短英文类型前缀和明确范围：

```text
feat: add deterministic movement transition
fix: reject stale match revisions
docs: clarify Web backend module boundary
chore: establish project skeleton
test: add golden replay for water timeout
```

只要会话修改了仓库，一次会话对应一次收口提交。若修改尚未达到可提交状态，应继续修正；确实受阻时在日志和交付说明中明确原因，不提交已知损坏的工作区。

## 验证基线

Rust 工具链可用后，默认运行：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

非 Rust 部分在对应工具链确定后，将验证命令补入根 `README.md` 和本文件。

当前零构建 Web 客户端修改时至少运行：

```bash
node --check clients/web/app.js
```

HTTP、领域契约或前端请求流程变化时，应启动本地后端完成对应 API 冒烟；UI 或规则变化再按风险增加关键路径、窄屏结构和确定性测试。
