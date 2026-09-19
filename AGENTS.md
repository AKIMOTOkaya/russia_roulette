# 仓库开发规则

本文件适用于整个仓库，记录当前开发协作约定。它不是不可修改的规格：用户后续明确改变产品方向、技术选型或流程时，以最新决定为准，并在同一会话同步更新本文件和相关项目说明。

## 开始开发前

- 阅读根目录 `README.md` 和 `docs/project/README.md`。
- 涉及玩法或 Core 时，阅读 `docs/rules/` 和相关测试。
- 涉及架构、模块或依赖时，阅读 `docs/architecture/` 与 `docs/project/modules.md`。
- 涉及本地 Web 时，阅读：
  - `docs/architecture/local-web-mvp-design.md`
  - `docs/rules/local-web-mvp-rules.md`
  - `docs/development/local-web-mvp.md`
  - `clients/web/README.md`
- 涉及 AstrBot 插件时，阅读 `docs/rules/astrbot-plugin-rules.md` 与 `clients/astrbot_plugin_russian_roulette/README.md`。
- 涉及部署时，阅读 `docs/project/deployment.md` 和 `deploy/` 内该项目自己的文件；不要修改其他应用或外部服务器影子目录，除非用户本次明确要求。
- 开始修改前检查 `git status`。已有修改默认属于用户，保留无关内容，不覆盖、不重置。

## 稳定架构边界

- 保持服务器权威：客户端只提交意图和并发版本，不自行裁定命中、地形、死亡、胜负、房间权限或随机结果。
- 保持单向依赖：

  ```text
  roulette-backend -> roulette-host -> roulette-core -> roulette-domain
                                     \--------------> roulette-domain
  clients/web      -> HTTP JSON      -> roulette-backend
  ```

- `roulette-domain` 只放可序列化领域契约，不读取网络、文件、时钟或随机源。
- `roulette-core` 保持确定性；系统时间、外部随机源、HTTP、房间和进程生命周期不得进入 Core。
- `roulette-host` 管理房间、成员、权限、Tab 与玩家映射、revision 校验和 Bot 调度，不认识 HTTP 或页面结构。
- `roulette-backend` 负责 Axum 路由、进程状态、静态资源、连接来源与本地服务设置，作为公共引擎库为不同运行入口提供统一服务。
- Bot 与真人走同一套 `PlayerCommand` 和 Core 规则入口，不直接修改状态。
- **本地局域网与公网中央服务器双入口与防重复纪律**：
  - 仓库构建区分 `roulette-local`（本地局域网模式，端口 8787，回环保护与临时创始人密码）与 `roulette-server`（公网中央服务器模式，端口 8080，放行反向代理公网流量与管理员凭据）两个独立进程，同时保留 `roulette-backend` 兼容入口。
  - **严禁代码复制**：业务逻辑、路由处理器、MCP 工具、状态机与仓储 100% 共享，不可因为构建两个进程应用而产生相似或重复的功能代码；所有通用功能必须修改共同兼容的库代码。
  - **差异压制在高层**：公网与局域网的区别（如反向代理放行、管理员 Token、用户认证）必须严格压制在最顶层（`ServerMode` 与 `UserIdentity` 转换层）。本地模式下通过高层将 `TabId` 转为 `IdentityKind::TemporaryTab`，未来公网认证直接转为 `IdentityKind::Authenticated`，下层业务用例与领域模型完全复用。


## 记录与事件语义

- 综合记录包含并列的 `Action`、`Event`、`Turn`、`Notification`。
- 行动可能诱发零到多个事件，但记录结构不声明一对一或父子因果。
- 机器人的选择属于行动，不因为操作者是 Bot 而归类为事件。
- 页面综合记录与左下事件检查器统一采用单列倒序排列，最新记录/新增项置于顶部（时间上在后发生的排在上面），无需向下滚动即可直观追踪最新战况；不要使用多列或从右向左的排列。
- 新增环境事件、定时事件或通知时，保持它们可以独立存在，为后续事件系统留出边界。

## 当前本地 Web MVP 基线

以下是当前实现，不是永久产品承诺；需要改变时同步代码、文档和本文件：

- 每个浏览器标签页用 `TabId` 视为一名本地真人；它只是本地关联标识，不是安全身份凭证。
- 后端持有多张内存房间，使用五位、大小写无关的房间号；关闭进程后不保留数据。
- 房间密码、创始人临时密码与局域网开关均为低安全要求的本地机制，不得直接当作公网安全设计。
- 真人和 Bot 在房间成员列表中使用同等席位展示；房主负责 Bot 管理、开局、转让和解散。
- 前端当前为 `clients/web` 下的零构建 HTML/CSS/JavaScript。仅为布局或小交互不要引入框架；若复杂度确实需要框架，先明确边界并同步运行、构建和验证文档。
- 当前信息架构为：品牌封面 → 以房间列表为主体的大厅 → 模态创建/加入/单人模拟/设置 → 房间与对局。
- 单人模拟复用正式房间链路（创建房间、添加 Bot、开局），不建立客户端专用规则分支。

## 命名与目录

- Rust package、二进制和 crate 目录统一使用 `roulette-*` 前缀。
- Cargo package 使用 `kebab-case`；Rust crate 和 module 使用 `snake_case`。
- 不创建职责不清的 `common`、`shared`、`utils`、`misc`、`manager` 或 `helpers` 模块。
- Rust 库放 `crates/`，Rust 应用放 `apps/`，客户端放 `clients/`，跨语言协议放 `protocol/`。
- 根目录不放普通业务源码；脚本、测试、部署和文档分别进入已有明确目录。
- 新模块按职责和依赖边界创建，不为未来设想提前生成空 crate。

## 修改范围与安全

- 只修改完成本次目标所需的文件，不顺手重构无关代码。
- 不提交密码、令牌、真实 `.env`、构建产物、缓存、临时文件或外部应用内容。
- 不执行破坏性 Git 操作，不覆盖用户已有修改。
- 未经明确要求，不推送、部署、发布或修改仓库外的服务器状态。
- 数据库尚未成为当前本地 MVP 依赖；接入数据库前先确认角色、schema、迁移和部署网络。

## 文档规则

- Markdown 是持续维护源稿；DOCX 只作为里程碑或明确要求下的评审快照。
- 当前事实和决定进入 `docs/project/`、`docs/architecture/`、`docs/rules/` 或 `docs/development/` 的对应文档。
- `docs/development-log/` 记录每次会话当时的目标、决定、验证和遗留事项；旧日志不重写。
- 决定改变时，新日志说明变化，并更新当前说明文档；不要通过修改历史日志伪装决定从未变化。

## 验证基线

Rust 工具链可用时默认运行：

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

修改当前零构建前端时至少运行：

```bash
node --check clients/web/app.js
```

并按改动风险补充验证：

- HTTP、Domain 或前端请求流程变化：启动本地后端做对应 API 冒烟。
- UI 结构或交互变化：检查桌面/窄屏结构、模态交互和关键用户路径。
- 规则变化：补充或更新 Core/Host 测试，验证确定性与 revision 不变量。
- 无法执行的检查必须在当次开发日志中说明原因和风险。

## 每次开发会话的交付要求

只要本次会话修改了仓库，就必须：

1. 同步更新 `docs/project/` 中受影响的项目说明。
2. 在 `docs/development-log/` 新建一篇按日期和序号命名的会话日志。
3. 执行与改动风险相称的格式检查、静态检查、测试或冒烟验证。
4. 检查 Git diff，确认没有敏感信息、构建产物、用户已有改动或无关文件。
5. 将本次会话修改整理为一次 Git 提交，并向用户报告提交哈希。

只读解释或诊断不创建空提交；若会话形成新的项目决定，则将决定写入文档和日志后提交。
