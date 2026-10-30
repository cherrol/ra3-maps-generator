# 广西 · 碧嶂回澜

已按选定的原游戏地形预览全新制作可玩地图 **[CHAI]碧嶂回澜T0.0.1**。地形从当前 PNG 重新读取，没有使用旧 NPZ 缓存、旧版本地图或其他地图布局作为制作依据。续用的布局与脚本均为本轮新规划成果。

[地图 ZIP](../../dist/广西/[CHAI]碧嶂回澜T0.0.1.zip) · [制作资料](../../assets/广西/[CHAI]碧嶂回澜T0.0.1/) · [交互坐标图](../../assets/广西/[CHAI]碧嶂回澜T0.0.1/coordinates/index.html) · [步骤与用时](histories/T0.0.1/production-steps.md)

## 原预览与地形保护

[真实地形](source.png) · [原游戏预览](preview.png) · [原边界版](preview--line.png) · [原行政中心版](preview--center--line.png)

四张公共 PNG 的内容和 CRC 均保持原样。原图主体 800×800 格映射到完整 840×840 网格的 (20,20)–(819,819)，北在上。原图的分层标签、水陆轮廓全部保持，仅按用户“不改地形，斜坡可以新增”新增 13 处 24 格宽的跨层坡道；15,651 格局部坡面高度改变，坡外高度差异为 0。水底沿用预览生成规则的 130，水面 200，没有额外岸坡或水深塑形。不可见的 20 格制作边框采用最近原图边缘延伸，单独记录，不冒充可见参考地形。

两片开放海湾跨出可玩框；原岸线保留为阻断海岸，不声称整圈两栖可通过。原图东西侧海域面积差异及非交战岸角细尖均保留；全部赠送避开这些位置，选定战斗通路满足净宽。设计资料仍保存在 [data/terrain-preview/](data/terrain-preview/)，本轮没有从其中的地形缓存构建地图。

## 本轮布局与验收

八席东西对置，六个玩家席位，固定苏联 P7 与日本 P8 位于东侧中间；48 矿，每席开局两矿、外围四矿，另有四油井、医院和车库。四组进攻对位为 P1–P5、P2–P7、P3–P8、P4–P6。

- 原生对象 174 个；实际赠送建筑分支 226 条，主备用组 110 组。
- 最终完整占地叠加后八条主次路线通过，最小保守净宽 10.2368 格。
- 基地至资源 432 条路径全部可达；96 条生产建筑分支均有基地到出口的静态接入路径。
- 13 坡全宽通行、完整引道避让、出生组、分级布矿和全部建筑四向核对通过；矿位退让保护坡道。
- 915 段游戏内岸线及全图层界、坡肩和材质字段均有同版记录；材质图明确为原生字段示意，并非真实贴图或游戏渲染。

[13 项独立验收](histories/T0.0.1/delivery-acceptance.json) · [原图一致性](histories/T0.0.1/preview-identity.json) · [坡道范围差异](histories/T0.0.1/ramp-only-rebuild.json) · [地形交接](histories/T0.0.1/terrain-transition-review.json) · [发布核验](histories/T0.0.1/release-verification.json)

完整制作源冻结为 [authoring/source.zip](authoring/source.zip)，配套完整布局、基准及对象身份表位于 `authoring/`；选定蓝图为 [histories/T0.0.1/layers.npz](histories/T0.0.1/layers.npz)，根目录 `layers.npz` 为同版镜像。全流水重建后五个游戏文件逐字节不变。可执行 `cargo run --release --locked -- build --map 广西` 复现。

本次完成原生和静态验收，未启动游戏或 GUI，未安装。此前含额外岸坡修改的本轮候选仍保留在本地撤回档案，禁止作为交付包；[交接状态](data/authoring-handoff/2026-10-30/handoff-status.json) 记录用户后续授权及修正结果。
