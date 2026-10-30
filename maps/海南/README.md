# 海南地形预览

当前保留真实地形、分层设计与预览资料。旧可玩配方、制作源、历史成品和发布包已按用户要求清理；本目录目前没有可直接构建的可玩地图。

[真实地形](source.png) · [游戏分层](preview.png) · [边界版](preview--line.png) · [行政中心版](preview--center--line.png)

设计可玩框为800×800格，四周各留20格边框，完整网格840×840，北在上。水域轮廓经过游戏设计整理；原始高程、水域、取景与处理记录保留在[data/terrain-preview/](data/terrain-preview/)，不以设计水面代替真实水域资料。

- [设计参数](data/terrain-preview/parameters.json)与[坐标声明](data/terrain-preview/coordinate-system.json)。
- [原始输入](data/terrain-preview/input.npz)、[当前分层数组](data/terrain-preview/terrain.npz)与[分层报告](data/terrain-preview/report.json)。
- [尺寸核对](data/terrain-preview/square-verification.json)、[水域统计](data/terrain-preview/water-report.json)及原始地理资料保留。

后续制作按[公共地形预览规范](../../docs/terrain_preview.md)与[公共地图约束](../../docs/map_rules.md)重新规划出生、对位进攻路线、沿线坡口和资源布局。现有预览不代表原生通行、可建造或完整交付验收已经通过。
