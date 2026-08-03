# 本地 Web 客户端

本目录是早期本地 Web MVP 的零构建前端，由 `roulette-backend` 编译时嵌入并在 localhost 提供。当前只使用原生 HTML、CSS 和 JavaScript，便于先验证玩法闭环；公网版选型不受此实现约束。

前端只负责页面、输入和表现状态，不保存权威 `GameState`，也不自行决定命中、死亡、随机事件或胜负。所有玩家与随机 Bot 的行为都由本地 Rust 进程裁定。页面将行动、事件、回合和通知作为同一时间线中的独立记录类别展示，不推断它们之间的因果绑定。

在仓库根目录运行：

```bash
cargo run -p roulette-backend
```

然后访问 <http://127.0.0.1:8787>。
