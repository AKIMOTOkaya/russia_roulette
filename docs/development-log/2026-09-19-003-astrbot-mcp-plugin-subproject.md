# 2026-09-19：创建 AstrBot 俄罗斯轮盘赌 MCP 插件子项目

## 用户目标

- 创建一个用于 AstrBot 机器人框架的子项目，通过 MCP 协议与俄罗斯轮盘后端游戏引擎交互；
- 遵循 AstrBot 插件市场规范（Star 架构，`metadata.yaml`，`_conf_schema.json`，`requirements.txt`，`logo.png` 等）；
- 支持未来独立上架 AstrBot 插件商城，自成独立 Git 仓库；
- 外层主仓库 `.gitignore` 完全忽略该子项目目录，避免产生嵌套 Git 仓库冲突；
- **核心业务边界约束**：
  - **绝不让 LLM 参与游戏内核与规则推演**，后端 Rust 游戏引擎全权保持 100% 服务器权威；
  - 初版 MVP 目标是让 QQ 用户通过机器人输入 `/指令` 直接参与游戏；
  - AstrBot 作为对局**主持人**与展示端，负责接收群聊/私聊用户输入、提交至后端 MCP 裁判接口，并将权威战况、Emoji 棋盘与戏剧性事件格式化展示给用户。

## 架构设计与实现

### 1. 独立 Git 仓库与外层隔离策略
- 依据 `AGENTS.md` 规范，客户端应用放置在 `clients/` 目录下：`clients/astrbot_plugin_russian_roulette/`；
- 在主仓库根目录 `.gitignore` 中追加 `/clients/astrbot_plugin_russian_roulette/`，确保外层 Git 完全忽略该子项目，杜绝嵌套子仓库报脏或 Submodule 冲突；
- 在子项目目录内部执行独立的 `git init`，创建独立的提交历史，默认分支设为 `main`，便于后续直接配置 `git remote add origin` 推送到独立的 GitHub 仓库。

### 2. AstrBot 插件规范落地
- **元数据 (`metadata.yaml`)**：
  - 标识符：`astrbot_plugin_russian_roulette`；
  - 规范配置 `display_name`、`version: 1.0.0`、`author`、`repo`、`desc`、`short_desc`、`tags`，满足 AstrBot Cloud 审核要求；
- **WebUI 可视化配置模式 (`_conf_schema.json`)**：
  - 支持在 AstrBot Web 管理界面调整：
    - `server_url`: 轮盘服务地址（默认 `http://127.0.0.1:8787`）；
    - `mcp_endpoint`: MCP 调用端点（默认 `/api/mcp/call`）；
    - `default_bots`: 默认对局 Bot 数量（默认 3）；
    - `admin_only`: 是否限制仅管理员可建房/解散；
    - `show_visual_board`: 是否打印 Emoji 字符战术棋盘；
    - `max_event_records`: 战报展示最新事件条数；
- **依赖与标识**：
  - `requirements.txt`: 声明 `aiohttp>=3.9.0` 与 `pydantic>=2.0.0`；
  - `logo.png`: 纯 Python 生成符合规范的 256x256 像素 1:1 左轮弹巢主题图标；
  - `LICENSE`: MIT 开源许可证。

### 3. MCP 异步通信层 (`mcp_client.py`)
- 实现 `RussianRouletteMcpClient` 异步客户端，对接后端 `/api/mcp/call` JSON-RPC 接口；
- 维护房间并发版本缓存（`_revision_cache`），写操作强制校验 `expected_revision`，发生并发冲突时自动重新获取最新版本并重试；
- 自动为所有写操作生成 UUID `idempotency_key` 保证网络重试幂等；
- 双传输适配：检测到 `aiohttp` 时优先使用全异步 ClientSession；在无外部依赖的极简环境下平滑回退至标准库 `urllib.request` + `asyncio.to_thread`，保证高可用性。

### 4. 战况与事件排版渲染器 (`renderer.py`)
- **主持人战报排版**：
  - 回合数、天气（暴雨/狂风/极寒/晴朗）、存活人数；
  - 玩家席位状态卡片（真人 👤 / 机器人 🤖，💚 存活 / 💀 淘汰死因，🛡️ 防弹盾完好度，坐标位置，当前行动方高亮 👉）；
  - 紧凑 Emoji 战术棋盘（地面 `⬜`、墙体 `🧱`、水洼 `🌊`、箱子 `📦`、玩家 `👤`）；
  - 最新戏剧性事件流水（枪击命中/空弹、防弹盾破损、地形掩体贯穿等）；
- **操作反馈排版**：
  - 裁判判定结果与终局获胜者庆祝公告。

### 5. 指令驱动主入口 (`main.py`)
- 继承 AstrBot `Star` 类，在 `__init__` 中注入 `AstrBotConfig`；
- 注册主指令 `@filter.command("rr")` 与中文别名 `@filter.command("轮盘")`；
- 包含会话房间记忆机制：自动记忆本群聊/私聊发起的活跃房间号，玩家执行行动时可省略重复输入房间号（如 `/rr 开火 上`、`/rr 移动 东`）；
- 全套子指令支持：
  - 房间管理：`帮助`、`状态`、`房间`、`创建`、`开始`、`战况`、`解散`；
  - 对局行动：`开火`（支持上/下/左/右/北/南/西/东/w/s/a/d）、`移动`、`等待`、`自戕`、`步进`；
  - 规则查询：`规则`。

## 验证与代码质量

1. **子项目独立测试套件**：
   - 编写 `test_standalone.py`（10 个测试用例，覆盖排版渲染、方向映射、MCP 客户端版本管理、指令派发与 Mock 完整建房/开局生命周期）；
   - 运行结果：`Ran 10 tests in 0.008s, OK`；
   - `python3 -m py_compile *.py` 全语法检查 100% 通过。
2. **Git 隔离与独立仓库验证**：
   - 外层主仓库 `git status` 仅显示受控修改（`.gitignore`、`clients/README.md`、`docs/project/modules.md`），无任何子项目红字或泄露；
   - 子项目内部 `git status` 与 `git log` 显示独立的 `main` 分支提交（commit `39978a0`），可随时推送至独立 GitHub 仓库。
3. **主仓库代码质量与测试基线**：
   - `cargo fmt --all --check`：100% 对齐；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：0 warnings；
   - `cargo test --workspace`：36 个测试全部通过；
   - `node --check clients/web/app.js`：前端语法校验通过。
