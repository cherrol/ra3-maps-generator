# 项目维护

地图制作与发布命令见 [首页](../../README.md)。维护工具不递增地图版本，不改变制作源或发布内容。

## 缓存清理

在所有构建结束后运行，默认只输出 JSON 计划：

```sh
cargo run --release --locked --example maintenance
cargo run --release --locked --example maintenance -- --apply
```

工具使用编译时的工程目录。请用 `--release` 或独立 `CARGO_TARGET_DIR` 编译运行，避免可执行文件位于待清理缓存内。Unix 支持执行清理；其他平台仅输出计划。

固定范围为：

| 缓存 | 可重新生成的来源 |
|---|---|
| `target/debug` | 当前 Cargo 工程 |
| `target/yunnan-preview` | `maps/云南/preview-tool` |
| `work/local-ramp/target` | `work/local-ramp` |
| `work/high-mountains/tool/target` | `work/high-mountains/tool` |
| `work/terrain-material-preview/target` | `work/terrain-material-preview/staging` |
| `work/rename-release/__pycache__` | 同目录 Python 源文件 |
| `work/china-terrain-preview/tools/__pycache__` | 同目录 Python 源文件 |

执行前核对 Cargo 缓存标记、源文件、符号链接和硬链接归属，再重新核对计划。出现变化会停止；逐个删除已核实文件，只移除空目录。`target/release`、地图、原始地理资料、制作快照、验收证据和发布包均不在清理范围。`work/` 包含历史输入与证据，不能整目录视作缓存删除。

受控测试输入使用 [素材恢复工具](reproducible_builds.md)，与缓存清理分开处理。恢复遇到内容不同的目标会停止，不能借恢复覆盖本地制作修改。

## 代码边界

功能目录、公共实现与接口约束见 [源码功能结构](../../src/README.md)。`src/pipeline/build/` 负责阶段、事务、组装、验收与分发，`placement.rs` 负责发布前的公共落点校验与证明；`src/pipeline/release/package.rs` 统一完整成品的解析和封装校验。`src/pipeline/frozen_source.rs` 集中读取并校验冻结源 ZIP、原生地图 CRC 和布局，坐标流水与坡口检查共用这一入口。

坐标公式与设计帧位于 `src/coordinates/frame.rs`，由 `coordinates::Frame` 保持对外接口。冻结源投影使用共享原生地图，仅在对象、地形或版本确实变化时复制并修改；完整成品仍经回读、空间/资源检查和字节保持回归。
