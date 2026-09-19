# 开发日志：AstrBot 插件审核规则入库与自动化安全检查合规整改

- 日期：2026-09-19
- 会话：007
- 状态：完成
- Git：与本日志同一提交；提交信息 `fix(astrbot): comply with marketplace audit rules and record plugin specs`

## 用户目标

针对 AstrBot 插件市场自动安全检查（`AstrPluginReviewer`）提出的驳回反馈进行全面整改：
1. **日志记录合规（硬性红线）**：彻底清除无条件 `import logging` 与 `logging.getLogger`，必须且仅能通过 `from astrbot.api import logger` 导入；清理回退分支中的内置 logging；
2. **规范沉淀**：深入研读 AstrBot 插件官方开发准则与市场 AI 审查反模式规则，在本地建立长效规范文档；
3. **全局合规自查与排查**：
   - 清查内部函数嵌套导入（如方法内部 `import re`）；
   - 清查同步阻塞 I/O（剔除 `urllib.request`，统一异步 `aiohttp`）；
   - 清查静默异常吞噬（消除裸 `except Exception: pass`，精准捕获 `McpError` 并留痕）；
   - 清查代码重复（DRY 原则，重构提取 `_extract_command_parts`，删除重复 exception 块）；
   - 修正元数据 URL 大小写一致性。

## 范围与假设

- 适用项目：`clients/astrbot_plugin_russian_roulette`（独立 Git 仓库）及未来 AstrBot 扩展；
- 远程环境：服务器容器 `miniaki-qqbot-astrbot-1`（AstrBot v4.28.1）；
- 本地测试账号：`2393120566`，严格限定测试群 `739995952`。

## 关键决定

1. **规范入库沉淀 (`docs/rules/astrbot-plugin-rules.md`)**：
   - 将日志唯一导入来源、模块顶部导入、异步网络 I/O、异常处理与日志留痕、DRY 设计模式、数据隔离与元数据发布规范完整编纂入库；
   - 在 `AGENTS.md` 和 `docs/project/modules.md` 中建立交叉索引。
2. **纯粹的框架导入与独立的单体测试 Mock Harness**：
   - 生产代码（`main.py`、`mcp_client.py`、`renderer.py`）不再包含任何用于本地单测的 `try...except ImportError` 容错或 fallback mock 类；
   - 所有 mock 测试环境（`sys.modules["astrbot"]`、`sys.modules["aiohttp"]`）100% 封装在 `test_standalone.py` 内部，保持插件源码的绝对纯净与高可读性。
3. **网络层全面异步化**：
   - 移除 `mcp_client.py` 中用于备用的 `urllib.request` 同步请求及 `HAS_AIOHTTP` 标志，只使用 `aiohttp.ClientSession`，完全满足插件市场对异步网络通信的要求。

## 修改内容

- `docs/rules/astrbot-plugin-rules.md`：
  - [NEW] 编写并发布 AstrBot 插件开发与市场审核规范全文。
- `AGENTS.md` & `docs/project/modules.md`：
  - 更新前置阅读要求与子项目说明，关联插件规范文档。
- `clients/astrbot_plugin_russian_roulette/mcp_client.py`：
  - 移除 `import logging`，改为 `from astrbot.api import logger`；
  - 移除 `urllib.request` / `urllib.error`，精简为标准异步 `aiohttp.ClientSession`。
- `clients/astrbot_plugin_russian_roulette/main.py`：
  - 移除 `import logging` 和内部所有 mock fallback 类；
  - 统一为标准 `from astrbot.api import ...` 与包内相对导入 `from .mcp_client import ...`；
  - 提炼 `_extract_command_parts`，消除指令分词与异常吞噬代码重复；
  - 移除 `_dispatch_command` 中多余的重复 `except` 块；
  - 移除方法内部 `import re`，统一置于文件头部；
  - 替换静默 `except Exception: pass` 为精准的 `except McpError` 并记录日志。
- `clients/astrbot_plugin_russian_roulette/renderer.py`：
  - 将 `import re` 提至文件顶部，移除函数内部局域 import。
- `clients/astrbot_plugin_russian_roulette/metadata.yaml`：
  - 修正 GitHub 仓库 URL 字母大小写为 `AKIMOTOkaya`。
- `clients/astrbot_plugin_russian_roulette/test_standalone.py`：
  - 在最外层装配 mock astrbot 与 mock aiohttp 模块环境；
  - 移除局域内联导入，全部使用文件头部导入；
  - 保持 19 项单元测试 100% 通过。

## 验证

1. **静态检查与编译**：
   - `python3 -m py_compile main.py mcp_client.py renderer.py test_standalone.py`：无任何语法或警告；
   - 全项目 grep `logging`、`urllib`、`requests`、`print`：均为 0 结果；
   - 清查函数级内联 import：除测试注入分支外全部清零。
2. **单元测试**：
   - 运行 `python3 test_standalone.py`：19 项测试全部通过（0.048s）。
3. **子仓库推送与部署**：
   - Git 提交并推送至 `origin/main` (`59c8580`)；
   - 运行 `./scripts/install-astrbot-plugin-to-server.sh --restart`：远端容器重启成功且适配器自动重连。
4. **真实群聊联调 (群 739995952)**：
   - `/rr 状态`：回执后端连接正常；
   - `/rr 帮助`：完整打印指令清单；
   - `/rr 创建 迷雾丛林 bots=2`：成功创建；
   - `/rr 开始`：精准拦截少于 3 人非法国度（`a match requires 3 to 6 room members`）；
   - `/rr bot 加`：成功增补至 3 名 Bot；
   - `/rr 开始`：成功装填并拆解 3 条独立消息分发；
   - `/rr 步进`：Bot 移动并推进至下一轮；
   - `/rr 解散`：成功强制解散清理房间。
5. **主仓库工具链质量验证**：
   - `cargo fmt --all --check`：通过；
   - `cargo clippy --workspace --all-targets --all-features -- -D warnings`：通过；
   - `cargo test --workspace`：38 项 Rust 测试全部通过；
   - `node --check clients/web/app.js`：通过。

## 遗留事项

- 用户可直接在 AstrBot 插件发布 Issue 中勾选重新审核或回复 `@astrpluginreviewer review`。
