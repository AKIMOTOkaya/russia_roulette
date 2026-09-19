# AstrBot 插件开发与市场审核规范

本文档记录 AstrBot 官方插件开发规范、插件市场自动化安全审查（`AstrPluginReviewer`）准则及上架准入要求。适用于 `clients/astrbot_plugin_russian_roulette` 及后续所有 IM 聊天机器人扩展。

---

## 1. 日志系统规范（硬性红线）

- **唯一合规导入**：所有需要记录日志的文件，必须且只能通过以下语句导入框架日志实例：
  ```python
  from astrbot.api import logger
  ```
- **严禁事项**：
  - **严禁**直接使用 Python 标准库 `logging`（即代码中禁止出现 `import logging` 或 `logging.getLogger(...)`）；
  - **严禁**在 `try...except ImportError` 中编写回退到标准 `logging` 库的容错逻辑；
  - **严禁**使用第三方日志库（如 `loguru`）或创建自定义的 `_DummyLogger` / `_StandaloneLogger` 类；
  - **严禁**直接使用 `print()` 打印调试信息。

---

## 2. 模块导入与依赖规范

- **顶部统一定义**：所有模块导入（无论是标准库、第三方库还是内部模块）必须位于文件顶部，**严禁在方法或函数内部动态导入**（如函数内部写 `import re`）；
- **明确的导入路径**：在插件包内部模块间引用时，统一使用标准的相对导入（如 `from .mcp_client import McpError`），避免模糊的顶层导入；
- **依赖声明**：所有非 Python 内置的第三方依赖必须在插件根目录的 `requirements.txt` 中显式声明，且不得包含平台冲突的私有包；
- **网络异步化**：
  - **严禁**使用同步阻塞库（如 `requests`、`urllib.request`）；
  - 网络请求必须且只能使用异步库（如 `aiohttp`、`httpx`）。

---

## 3. 异常处理与健壮性规范

- **严禁静默吞噬异常**：严禁在代码中出现 `except Exception: pass`。吞掉异常会掩盖运行时真实错误与环境崩溃；
- **精准捕获**：尽可能捕获具体异常类型（例如 `McpError`、`aiohttp.ClientConnectorError`、`ValueError`、`AttributeError` 等）；
- **日志留痕**：捕获异常后若无法向上抛出，必须使用 `logger.warning(...)`、`logger.error(...)` 或 `logger.debug(...)` 记录关键上下文信息。

---

## 4. 代码架构与反模式禁令

- **单一类封装**：所有指令与事件处理逻辑必须定义在继承自 `Star` 的主插件类内部；
- **严禁类外动态挂载**：严禁在类定义外部定义函数后再通过 `PluginClass.method = external_func` 动态绑定；
- **严禁变量命名遮盖内置关键字**：禁止将 Python 内置函数/关键字作为变量名（如 `id = ...`、`type = ...`、`filter = ...`、`input = ...`）；
- **DRY 原则（Don't Repeat Yourself）**：重复的逻辑（如会话提取、消息反查、指令前缀裁切）必须提炼为清晰的私有辅助函数（如 `_extract_message_str`），禁止多处复制粘贴。

---

## 5. 数据存储与安全边界

- **数据目录隔离**：
  - 插件若有持久化文件存储需求，必须使用 `StarTools.get_data_dir("plugin_name")` 或存入 `data/` 目录；
  - **严禁**直接写入插件源码自身所在目录，防止插件更新时被整体覆盖抹除；
  - **严禁**硬编码本地绝对路径（如 `/root/...`、`/home/...`）。
- **零安全风险**：
  - 严禁出现反向 Shell、子进程随意执行未经校验的系统命令（`os.system`、`subprocess.Popen`）；
  - 严禁包含凭证窃取、令牌外泄、持久化后门代码；
  - 严禁对 LLM 审查员或模型上下文施加提示注入（Prompt Injection）。

---

## 6. 元数据与发布包规范

- **`metadata.yaml` 完备性**：必须包含以下完整字段：
  - `name`: 插件唯一标识（全小写、无空格，以 `astrbot_plugin_` 开头）；
  - `display_name`: 中文展示名称；
  - `version`: 符合 SemVer 规范的版本号（如 `1.0.0`）；
  - `author`: 插件作者名字；
  - `repo`: 对应开源代码仓库 URL；
  - `short_desc`: 简短单行说明；
  - `desc`: 详细 Markdown 使用文档；
  - `tags`: 检索标签数组。
- **发布包大小限制**：
  - 上架与 CI/CD 流水线限制单个 `.zip` 压缩包大小不得超过 **16MB**；
  - 必须排除 `.git/`、`__pycache__/`、临时单测输出与无关二进制。
