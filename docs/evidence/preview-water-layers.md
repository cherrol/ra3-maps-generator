# 全系列预览小水域归层记录

本报告记录先前全系列归层批次；其中 11 地区后续整理了细长地形与局部水域，当前数组和岸线验收以[预览修复报告](preview-intrusion-repair-2026-10-30.md)及各图当前报告为准。原批次证据保留，不作为新数组的校验和。

2026-10-29 按[小水域单层包裹规则](../spec/terrain_preview.md#小水域的单层包裹)核对全部 34 张设计预览，并使用公共原生 Rust 渲染入口重新生成 135 张图片：34 张真实高程图、34 张分层图、34 张边界图及 33 张已有行政中心图。河南沿用三张基础图。

## 范围和设计选择

本批共识别 86 个水域连通分量。57 处完全位于游戏框内的闭合水域纳入归层验收，各图逐处记录面积及占比；它们为 3–13,463 格，最大占可玩范围约 2.10%，本轮均作为局部小水域处理，不设新的公共面积门槛。29 处开放或跨框水体单列为本轮闭合包裹不适用，不把画框当作人工岸线；其接岸分层保持原样。

对 13 处原先跨层的水域，以**修改前完整岸线的四邻共享边长度**选择最多接触层。本批共享边均为 10 世界单位，没有并列情况。以全周岸边陆地格为中心取半径 12 格的圆盘并集，在可玩陆地内局部归层，将层界移出岸线；12 格仅为本批设计选择，不是公共下限。水域掩码、真实高程、地理边界、取景、分层切点和岸带外层级保持不变。

57 处适用水域均已回读验证全周陆地侧只接触归属层，各岸段接入未修改的同层陆地主体；包括已满足单层包裹的 44 处。预览的单层包裹通过，不替代之后可玩制作时的整圈缓坡、坡度和双向水陆导航验收。

## 实际修改

仅下列 10 张图需要改变实际分层，共修改 10,382 格陆地。其余 24 张图也完成重绘，现有图片保持原字节。

| 地图 | 跨层水域数 | 归属层（按水体扫描顺序） | 修改陆地格数 | 逐水域证据 |
|---|---:|---|---:|---|
| 北京 | 1 | 第 2 层 | 1,590 | [验收](../../maps/北京/data/terrain-preview/water-layer-verification.json) |
| 广东 | 1 | 第 1 层 | 886 | [验收](../../maps/广东/data/terrain-preview/water-layer-verification.json) |
| 新疆 | 3 | 第 3、2、3 层 | 2,302 | [验收](../../maps/新疆/data/terrain-preview/water-layer-verification.json) |
| 江西 | 1 | 第 2 层 | 806 | [验收](../../maps/江西/data/terrain-preview/water-layer-verification.json) |
| 河南 | 1 | 第 2 层 | 907 | [验收](../../maps/河南/data/terrain-preview/water-layer-verification.json) |
| 浙江 | 1 | 第 2 层 | 270 | [验收](../../maps/浙江/data/terrain-preview/water-layer-verification.json) |
| 海南 | 1 | 第 2 层 | 766 | [验收](../../maps/海南/data/terrain-preview/water-layer-verification.json) |
| 湖北 | 2 | 第 2、2 层 | 2,020 | [验收](../../maps/湖北/data/terrain-preview/water-layer-verification.json) |
| 湖南 | 1 | 第 3 层 | 570 | [验收](../../maps/湖南/data/terrain-preview/water-layer-verification.json) |
| 黑龙江 | 1 | 第 1 层 | 265 | [验收](../../maps/黑龙江/data/terrain-preview/water-layer-verification.json) |

## 输入、证据与复现

每图的 `data/terrain-preview/water-layer-verification.json` 绑定候选地形 CRC 和全部图片 CRC，列出全部水体、原接触长度、归属层、整理后接触长度、岸边连通结果及修改范围口径。修改图还保存：

- `raw/before-water-layer-enclosure/terrain.npz`、`report.json`、`parameters.json`：调整前快照，保留原接触量及来源引用。
- `water-layer-reference.npz`、`water-layer-reference-report.json`：已整理的分层与设计高度，供当前参数引用；普通预览命令可直接复现。
- 当前 `terrain.npz`、`report.json` 和全部已有图片：从上述输入经公共原生生成入口产出并回读核对。

批处理工具为 [preview_water_layers.rs](../../xtask/preview_water_layers.rs)，默认只检查；显式 `--apply` 才更新参考并重绘，支持 `--map <真实地形名>` 限定范围。原始投票始终从保留快照读取，重复执行不会重新以已修改岸线投票；发现快照之后的其他分层修改、并列归属、岸带冲突或原生结果无法复现目标时停止。

```sh
cargo run --release --locked --example preview-water-layers
RA3MAP_FONT=/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc cargo run --release --locked --example preview-water-layers -- --apply --radius 12
```

单图离线重绘继续使用原入口，例如：

```sh
RA3MAP_FONT=/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc cargo run --release --locked -- terrain-preview 北京 maps/北京/data/terrain-preview/input.npz
```

## 验证

原生批处理逐图核对候选全部 NPZ 数组与目标一致，并检查全周归属、岸边主体连接、修改范围、图片身份及参考图保持。另以独立 Python 标准库解码 NPZ/PNG 回读，重新提取水体并统计接触边，检查全部 21,760,000 个分层预览像素、135 张 PNG 的尺寸与 CRC、设计高度与层号对应，以及所有修改格均位于记录岸带内。34 张真实高程参考图及其余 24 张未改分层图的现有图片均保持原字节。

[独立回读报告](preview-water-layers.verification.json)保存逐图候选 SHA256、证据路径和汇总。251 个可玩地图配置、原生来源及历史文件通过前后 SHA256 比对；本轮更新设计预览，不递增可玩版本。

批处理工具的接岸长度选择、全周归层、开放水体保持和对角／并列处理行为测试通过；`cargo fmt --all --check` 与该工具的 Clippy 检查通过。
