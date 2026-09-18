# 2026-09-19：区分局域网与公网双入口及高层兼容性架构实现

## 用户目标

- 实现区分本地局域网启动（`roulette-local`）和公网中央服务器（`roulette-server`）的不同入口进程版本；
- 为后续中央服务器版本可能引入的用户认证做准备，把局域网运行与公网运行的差异严格压制在高层（例如本地单标签运行转化为临时 identity 抽象）；
- **核心纪律**：修改共同兼容的代码，在上层作兼容性区分，严禁因为构建出两个应用就不断产生相似的相同功能代码；必须写入文档与规则，防止后续遗忘。

## 设计与实现细节

### 1. 统一身份抽象与运行模式规范 (`crates/roulette-domain`)

- **`ServerMode` 枚举**：
  - `Local`：本地局域网单机模式；
  - `Public`：公网中央服务器模式；
  - 提供 `description(&self) -> &'static str` 友好的中文字符串展示。
- **`UserIdentity` 统一身份结构**：
  - 拥有 `id: String`、`display_name: Option<String>`、`kind: IdentityKind`；
  - `IdentityKind::TemporaryTab`：本地模式下的临时标签会话标识；
  - `IdentityKind::Authenticated`：未来公网中央服务器下经过认证的用户账号；
  - 实现 `From<&TabId>`, `From<TabId>`, `From<&UserIdentity>`, `From<UserIdentity>`, `AsRef<str>` 等双向无缝转换，下层对局与房间用例透明兼容。

### 2. 宿主与服务层多态感知 (`crates/roulette-host`)

- `ServerSettings` 补充 `server_mode: ServerMode` 字段；
- `GameService` 增加 `server_mode: ServerMode` 字段及 `with_server_mode` / `new_public(manager, admin_token)` 构造辅助；
- `get_server_settings` 感知当前运行模式：公网模式下默认放行反向代理流量且不要求普通客户端本地认证。

### 3. 后端服务引擎库与双二进制入口 (`apps/`)

- **共享引擎库架构 (`apps/roulette-backend`)**：
  - 配置 `[lib]`（`roulette_backend`）与 `[[bin]]`（`roulette-backend`）；
  - 将所有 Axum 路由、HTTP 处理器、MCP 工具、中间件、DTO、前端静态资源全部保持在共享库内；
  - 升级 `lan_guard` 为 `access_guard`：公网模式直接放行流量，本地模式检查回环或局域网开启状态；
  - 在 `handlers.rs` 顶层提供 `resolve_identity` 转换器，为未来中心鉴权（Token/Session）提供唯一的接入插座；
  - `lib.rs` 统一提供 `ServerConfig`、`run_server`、`run_local`、`run_public_server`、`run_from_env`；
  - 保留 `roulette-backend` 默认二进制，通过环境变量或参数自适应启动，确保既有 Docker/Compose 模板完全兼容。
- **两个极简组装根应用 (Composition Roots, 零重复代码)**：
  - `apps/roulette-local`：依赖 `roulette-backend`，`main.rs` 仅 10 行，执行 `roulette_backend::run_local()`（默认端口 8787，本地回环防护，临时创始人密码）；
  - `apps/roulette-server`：依赖 `roulette-backend`，`main.rs` 仅 10 行，执行 `roulette_backend::run_public_server()`（默认端口 8080，放行公网流量，管理员凭据）；
  - 根目录 `Cargo.toml` 注册 workspace 依赖 `roulette-backend`。

### 4. 前端轻量感知 (`clients/web/app.js`)

- `renderServerSettings` 感知 `settings.server_mode`：
  - 若为 `public`，徽章显示“公网服务器”，隐藏/锁定局域网切换开关，提示公网模式已就绪；
  - 若为 `local`，保持原有的本地创始人认证与局域网访问控制交互。

### 5. 架构纪律入规与项目文档同步

- 在 `AGENTS.md` 的“稳定架构边界”中永久写入双入口与防代码重复纪律；
- 在 `docs/project/modules.md` 中更新 Cargo 模块包矩阵与“双入口模式与无重复代码设计”架构节。

## 验证与代码质量

1. **自动化测试与静态检查**：
   - `cargo test --workspace`：38 项测试全部通过（26 Core + 8 Host + 2 Domain + 2 Backend Integration）；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 告警 0 错误；
   - `cargo fmt --all --check`：格式完全合规；
   - `node --check clients/web/app.js`：前端语法校验通过。
2. **冒烟与多入口验证**：
   - `roulette-local` 启动测试：成功绑定 `127.0.0.1:18787`，`/api/server/settings` 返回 `{"server_mode":"local","lan_exposed":false}`；
   - `roulette-server` 启动测试：成功绑定 `0.0.0.0:18080`，`/api/server/settings` 返回 `{"server_mode":"public","lan_exposed":true}`。
