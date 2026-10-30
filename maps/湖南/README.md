# 九州大地·群峰抱泽

## 800×800 正方形预览（2026-10-29）

当前可玩范围 **800×800 格**，四周各 20 格制作边框，完整设计网格 **840×840 格**。保留原地理取景和北向，宽高分别重采样；图例与留白不计入格数。已沿用现有陆地层号、海拔切点及设计岸线，四张预览同步重绘，行政中心按原 WGS84 经纬度重新定位。PNG 画布为 1104×1002 像素。

[游戏分层](preview.png) · [边界版](preview--line.png) · [行政中心版](preview--center--line.png) · [尺寸与图片核对](data/terrain-preview/square-verification.json)。调整前分层、报告、参数和旧统计保存在 `data/terrain-preview/raw/before-square-800/`；下文早期整理的处理格数按当时网格记录。

## 2026-10-29 游戏预览水域整理

按[公共水域整理规则](../../docs/spec/terrain_preview.md#游戏水域整理)，相邻分叉与碎水优先就近合并、圆整为宽阔闭合水面，无法合理合并的细小水域并入附近陆地层；开阔海域保留并清理窄水道。当前基础图与中心标点图均使用设计水域。真实高程、边界、取景和原始来源掩码保持一致，岸线调整不作为真实水体面积资料。

[当前预览](preview--line.png) · [中心标点版](preview--center--line.png) · [本图整理规则](data/terrain-preview/water-shaping.json) · [水域统计](data/terrain-preview/water-report.json) · [验收记录](data/terrain-preview/water-verification.json)

整理前完整输入保存在 `data/terrain-preview/raw/reference-before-water-shaping.npz`；原始筛选政策和统计另存 `raw/before-water-shaping-*`。下列早期水域说明保留为来源记录，当前水面以本节链接为准。

2026-10-29 根据真实高程制作的游戏地形设计预览，尚未布置出生点、资源、建筑、斜坡或生成可玩地图。

[原始真实高程](source.png) · [游戏分层](preview.png) · [边界版](preview--line.png)

可玩范围 **800×800 格**，正方形，四周另留 20 格制作边框。北朝上。主体地理边界只作参考，矩形内的周边真实地形也纳入游戏范围。三图取景、朝向和水域显示一致。

全国基础海拔切点为 200、500、1500、4000 米；本图最终切点为 **200、500、1500、4000 米**。当前保留 **3 个陆地层级**，实际层号为 1、2、3；对应游戏高度为 210、280、350。图例省略缺失层，碎层并入相邻层，不足两层时按当地真实高程补分，高原提高第五层门槛以保留可玩台地。

河流不作为预览水域，保留真实海洋和完整参考轮廓估算面积至少 80 平方公里的湖泊、水库。细窄海域水道按既定规则整理，不放大湖泊。真实高程保留不变，水域着色采用同一筛选结果。

[参数](data/terrain-preview/parameters.json) · [实际分层及调整报告](data/terrain-preview/report.json) · [水域轮廓与面积](data/terrain-preview/water-policy.json) · [数据来源](data/terrain-preview/sources.json) · [核对记录](data/terrain-preview/verification.json)

复现：

```bash
cargo run --release --locked --offline -- terrain-preview 群峰抱泽 maps/湖南/data/terrain-preview/input.npz
```

真实高程来自 Mapzen / AWS Terrain Tiles；SRTM、GMTED2010 致谢 USGS。海洋和湖泊参考来自 Natural Earth，部分水库补充 © OpenStreetMap contributors 数据。边界来自 DataV GeoAtlas，近似反算到 WGS84 后对齐。采样高程极值和矢量估算面积不等同于实测值。

## 行政中心标点预览

[查看中心标点与边界图](preview--center--line.png)：地级行政中心，图中标注 14 处，含名称与必要的引线。点位及来源见 `data/terrain-preview/centers.json`，核对记录见 `centers-report.json`。
