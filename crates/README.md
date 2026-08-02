# Rust 库

这里存放可被多个程序复用的 Rust crate。库不得包含具体进程的启动逻辑。

当前建立 `roulette-domain`、`roulette-core` 和 `roulette-host` 三个边界骨架。后续按实际开发顺序增加协议、存储与回放适配器，避免提前创建无职责的空 crate。
