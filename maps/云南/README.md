# 九州大地·层峦织彩

## 800×800 正方形预览（2026-10-29）

当前可玩范围 **800×800 格**，四周各 20 格制作边框，完整设计网格 **840×840 格**。保留原地理取景和北向，宽高分别重采样；图例与留白不计入格数。已沿用现有陆地层号、海拔切点及设计岸线，四张预览同步重绘，行政中心按原 WGS84 经纬度重新定位。PNG 画布为 1104×1002 像素。

[游戏分层](preview.png) · [边界版](preview--line.png) · [行政中心版](preview--center--line.png) · [尺寸与图片核对](data/terrain-preview/square-verification.json)。调整前分层、报告、参数和旧统计保存在 `data/terrain-preview/raw/before-square-800/`；下文早期整理的处理格数按当时网格记录。

## 2026-10-29 游戏预览水域整理

按[公共水域整理规则](../../docs/spec/terrain_preview.md#游戏水域整理)，相邻分叉与碎水优先就近合并、圆整为宽阔闭合水面，无法合理合并的细小水域并入附近陆地层；开阔海域保留并清理窄水道。当前基础图与中心标点图均使用设计水域。真实高程、边界、取景和原始来源掩码保持一致，岸线调整不作为真实水体面积资料。

[当前预览](preview--line.png) · [中心标点版](preview--center--line.png) · [本图整理规则](data/terrain-preview/water-shaping.json) · [水域统计](data/terrain-preview/water-report.json) · [验收记录](data/terrain-preview/water-verification.json)

整理前完整输入保存在 `data/terrain-preview/raw/reference-before-water-shaping.npz`；原始筛选政策和统计另存 `raw/before-water-shaping-*`。下列早期水域说明保留为来源记录，当前水面以本节链接为准。

层峦之间晴雨交替，林色随坡铺展，流云越岭舒卷。

## 地形预览（全国统一分层）

2026-10-28 按全国基础海拔切点 **200、500、1500、4000 米**重新生成三张同取景预览。游戏范围为 **800×800 格**，四周制作边框各 20 格，北朝上；省界只作参考，矩形内的周边真实地形一并切层。

当前保留 **5 个陆地层级**（第 1、2、3、4、5 层），对应游戏高度 **210、280、350、420、700**；水域单独计算。没有的高度层省略，碎层并入相邻层；不足两层时按当地真实高程补分并记录切点；高原提高高山门槛，宽阔高原面保留为可玩台地。三条规则及算法见[公共预览规范](../../docs/spec/terrain_preview.md)。本图本轮无需局部调整海拔切点。

[真实地形](source.png) · [游戏分层](preview.png) · [边界版](preview--line.png)

本轮参数、原始输入与当前结果均在 `data/terrain-preview/`：

- [参数](data/terrain-preview/parameters.json)：公共入口优先读取。
- [输入](data/terrain-preview/input.npz)：真实高程、水域、主体边界与 WGS84 经纬度取景。
- [当前结果](data/terrain-preview/terrain.npz)：真实高程、分层、游戏高度及水域/省界掩码。
- [分层报告](data/terrain-preview/report.json)：最终切点、实际层数与各层面积。
- [核对记录](data/terrain-preview/verification.json)：三图取景、水域、完整边界和层高一致性。

这是地形设计预览。现有可玩地图的配方、布局、版本、定稿地形数组和制作源均保持原样；旧的根目录 NPZ 和历史报告继续保留供回查，不代表本轮预览结果。本轮已去除河流，仅保留海洋及完整轮廓估算面积不少于 80 平方公里的湖泊、水库；第五层禁行设计仍需后续制作地图落实。

此前云南预览资料仍保留在 `dem.npz`、`preview-layers.npz`、`game-area.npz`、`context.npz` 及原有报告中。原始高程来自 Mapzen / AWS Terrain Tiles（SRTM、GMTED2010 致谢 USGS），水域来自 © OpenStreetMap contributors。此前独立预览工具及根目录 `preview.json` 为历史输入；当前复现使用下列公共命令。

```bash
cargo run --release --locked --offline -- terrain-preview 层峦织彩 maps/云南/data/terrain-preview/input.npz
```

水域筛选：海洋取真实海洋轮廓，河流不参与水域掩码。湖泊、水库按完整参考轮廓估算面积筛选，省界及游戏外框不作为水体面积边界。三张图共用筛选结果，真实高程未改动，原始水域输入保留在 `data/terrain-preview/raw/reference-before-water-filter.npz`。详见[水域取舍](data/terrain-preview/water-policy.json)和[保留水体格数](data/terrain-preview/water-report.json)。

## 行政中心标点预览

[查看中心标点与边界图](preview--center--line.png)：地级行政中心，图中标注 16 处，含名称与必要的引线。点位及来源见 `data/terrain-preview/centers.json`，核对记录见 `centers-report.json`。
