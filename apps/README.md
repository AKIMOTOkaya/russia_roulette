# 可执行程序

这里存放 Rust 可执行程序及其组合配置，例如当前兼任本地 HTTP 服务的 `roulette-backend`、未来的 `roulette-node`、`roulette-cli` 和 Bot Runner。

可执行程序负责读取配置、组合 crate、启动运行时和处理退出信号，不在这里重新实现游戏规则。
