# 2026-08-18：维护仓库开发规则

## 用户目标

- 根据当前项目状态维护一次根目录 `AGENTS.md`。
- 保持规则便于后续继续修改，不把当前阶段实现写成不可变规格。

## 范围

- 重整仓库级开发前置阅读、架构边界、当前本地 Web 基线、验证和交付规则。
- 同步 `docs/project/development-workflow.md` 对 `AGENTS.md` 的定位和维护方式。
- 不修改业务代码、玩法、HTTP 接口、前端页面或部署配置。

## 关键决定

1. `AGENTS.md` 明确区分稳定不变量与当前 MVP 基线。服务器权威、确定性 Core、单向依赖和分类记录语义保持稳定；Tab ID、原生前端、本地密码和现有界面层级允许随用户后续决定调整。
2. 用户最新明确决定优先于文件中的阶段性假设；改变方向时在同一会话同步规则、当前项目说明和新日志。
3. 增加按任务类型的文档阅读路由，避免每次加载所有文档，同时确保玩法、Web 和部署任务读取对应来源。
4. 增加当前零构建前端的 `node --check clients/web/app.js` 基线，并明确 HTTP、UI 和规则变化各自需要的风险匹配验证。
5. 保留“修改仓库即同步项目说明、写日志、一次提交”的既有交付约定，并补充保护用户已有修改、禁止隐式部署和敏感信息检查。

## 修改内容

- `AGENTS.md`：按当前架构和实现重写为可持续维护的仓库协作规则。
- `docs/project/development-workflow.md`：新增 `AGENTS.md` 维护原则，收紧一次会话一次提交的表述，并补充 Web/API 验证基线。
- 本日志：记录本次规则维护的范围和后续可变性。

## 验证结果

- 文档结构人工检查和 `git diff --check`：通过。
- `cargo fmt --all --check`：通过。
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过。
- `cargo test --workspace`：通过，共 12 项单元测试和全部文档测试。
- Rust 与 Web 业务代码未修改，因此未执行本地服务或浏览器冒烟。

## 遗留事项

- 后续技术选型、前端框架、公网身份或数据库方案确定后，应再次维护 `AGENTS.md` 的“当前本地 Web MVP 基线”。
- 若新增子目录级特殊规则，可在对应目录添加更窄作用域的 `AGENTS.md`，避免根规则持续膨胀。

## 提交说明

计划提交信息：`docs: refresh repository development rules`
