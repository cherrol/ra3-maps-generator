# RA3 地图工坊（ra3maps）

红警3（Red Alert 3 / SAGE）自定义地图的**可复现构建工程**：直接生成 `.map` 二进制，
不依赖 WorldBuilder 手工绘制。当前产出：`[CHAI]八桂大地`（以广西壮族自治区地形为蓝图的 8 人 4v4 地图）。

## 一、目录结构

```
ra3maps/
├─ README.md                本文件
├─ build.cmd                一键构建入口（cmd，推荐）
├─ build.ps1                一键构建入口（PowerShell 包装）
├─ dist/                    【发布包】每张地图一个总目录，里面按版本存放
│   └─ [CHAI]八桂大地/
│       ├─ [CHAI]八桂大地T0.0.8/        ← 整个文件夹可直接拷进 Maps\
│       │   ├─ [CHAI]八桂大地T0.0.8.map
│       │   ├─ [CHAI]八桂大地T0.0.8_art.tga
│       │   └─ map.str
│       ├─ [CHAI]八桂大地T0.0.8.zip     ← 同内容压缩包（分发用）
│       └─ …（历史版本 T0.0.1 ~ T0.0.7）
├─ assets/                  【工程产物】每张地图一个总目录，里面按版本存放
│   └─ [CHAI]八桂大地/
│       ├─ [CHAI]八桂大地T0.0.8/        ← 当前版本
│       │   ├─ [CHAI]八桂大地T0.0.8.map / _art.tga / map.str   （游戏文件）
│       │   ├─ preview.png              总览预览图
│       │   ├─ README.md                该版本的说明（自动生成）
│       │   └─ source/                  生成物与自检图（terrain.npz、各类 preview）
│       └─ …（历史版本 T0.0.1 ~ T0.0.7）
├─ maps/                    【配方】一张地图的全部"参数与素材"
│   ├─ .default             默认配方 id
│   └─ bagui_dadi/
│       ├─ config.py        配方：尺寸/高程/水域/出生点/河流/纹理/对象布局/文本
│       ├─ README.md        该图的散文说明（会被合进发布包说明）
│       ├─ version.json     版本号状态与历史
│       └─ data/            配方输入（guangxi_boundary.json 广西 14 地级市边界）
├─ templates/               【骨架】可复用的 .map 模板
│   └─ 8p_template.map      8 人模板（含"房间 6 位 + 2 固定 AI"脚本系统）
├─ tools/                   【工具链】与具体地图无关
│   ├─ project.py           配方/路径解析（--map / RA3MAP_ID / maps/.default）
│   ├─ ra3map.py            .map 读写库（字符串表、chunk、属性字典、对象）
│   ├─ refpack.py           RefPack 解压（含社区地图的 0x80 变体）
│   ├─ refpack_enc.py       RefPack 压缩（LZ77，带回溯引用）
│   ├─ terrain_gen.py       地形生成（v11 分层蓝图 / 层间斜坡 / 出生点 / 资源）
│   ├─ map_gen.py           地图组装（骨架 + 地形 -> .map / _art.tga / map.str）
│   ├─ accept.py            构建验收（矿规则 / 矿场可建造 / 斜坡 / 轴向，失败则构建失败）
│   ├─ check_passability.py 可通行性与出生点连通性自检（水上基地不要求陆路连通）
│   ├─ verify_map.py        成品校验 + 回读渲染（build_view / art_preview）
│   ├─ make_preview.py      生成发布用预览图（中文标注+图例）
│   ├─ make_readme.py       生成发布包内的地图说明（自动表格 + 配方散文）
│   ├─ make_dist.py         发布打包：只把游戏文件放进 dist/<地图名>/（可出 zip）
│   ├─ version.py           版本号状态与递增（测试 T / 正式 V）
│   ├─ build_all.py         一键流水线（可 --install 安装到游戏）
│   ├─ layers5_v10.py       仅提供五档配色常量供 `work/*` 引用（v4~v9 已清理）
│   └─ …                    辅助/逆向脚本（scan_maps、dump_scripts、tex_tokens …）
├─ lib/opensage/            【参考】OpenSAGE 源码（格式权威参考）
├─ docs/                    【文档】格式规范（ra3_map_format_spec.md）与会话状态
└─ work/                    【本机诊断，不进版本库】蓝图导出与旧脚本；验收以 tools/accept.py 为准
```

**验收在 `tools/accept.py`**，已接进 `build_all` 的 `accept` 阶段。`work/` 被 `.gitignore` 忽略，只放本机诊断和蓝图导出脚本；其中 `check_ore_rules.py` 等是转到 `accept.py` 的兼容入口。

**⚠ 轴向约定（务必先读）**：内部数据第 0 行 = 最北；写进 `.map` 时高度场与物件 y 翻到游戏帧（第 0 行 = 屏幕下方）。显示图（art.tga、预览）北朝上、海在下方。判据是「海在哪一半」，不要用形状 IoU。权威说明在 `docs/SESSION_STATE.md` 第 1 节，检查在 `tools/accept.py` 的 axis 项。


**职责划分**：`maps/` 只放"这张地图是什么"（配方与素材）；`tools/` 放"怎么造"（通用算法）；
`assets/` 放"造出来的东西"；`dist/` 放"能直接进游戏的东西"。新增地图只动 `maps/` 与产物目录。

**产物目录规则**：`assets/<地图名>/<地图名+版本>/` 与 `dist/<地图名>/<地图名+版本>[/.zip]` ——
一张地图的所有版本、所有相关文件（地图、美术、文本、预览、说明、自检数据、发布包）
都在同一个以地图名命名的总目录下，版本各占一层；`dist/<地图名>/<地图名+版本>/` 整包可直接拷进游戏。

## 版本号规则

* **测试版**：`<地图名>T<major>.<minor>.<patch>`，从 **`T0.0.1`** 起，**每次修改 +0.0.1**
* **正式版**：`<地图名>V<major>.<minor>.<patch>`，发布时置为 **`V1.0.0`**，之后**每次 +0.0.1**
* 递增按三位十进制进位：`T0.0.9 → T0.1.0 → … → T0.9.9 → T1.0.0`
* 版本号参与**发布名**（游戏内地图名 = 文件夹名 = .map 文件名），因此每个版本在房间里是独立一张图；
  同时写进 `map.str` 文本与预览图标题
* 状态与历史存在 `maps/<id>/version.json`（谁、哪天、改了什么）

```powershell
# 查看当前版本与历史
& $py tools\version.py --map bagui_dadi --show

# 改动后：测试版 +0.0.1 并重新出包（--bump 在构建前递增）
& $py tools\build_all.py --map bagui_dadi --bump --note "调整中路矿脉" --install

# 发布正式版：转 V1.0.0；再次构建即为 V1.0.0
& $py tools\version.py --map bagui_dadi --release --note "首个正式版"

# 正式版继续 +0.0.1
& $py tools\build_all.py --map bagui_dadi --bump --release --note "平衡性调整" --install

# 需要时直接指定版本
& $py tools\version.py --map bagui_dadi --set T0.0.5 --note "回滚到该版本"
```

> 约定：**每次修改地图内容（配方/地形/对象布局）都必须递增版本号**，纯工具链改动不必递增。
> `--bump` 若构建失败会把版本号滚回，并删掉这次新产生的版本目录。
> 当前版本见 `maps/bagui_dadi/version.json`（现为 **T1.9.8**）。

## 二、环境

* Python：`C:\Users\cherrol\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\python\python.exe`
  （依赖 numpy、Pillow，随 DSH 运行时提供）
* 游戏地图目录：`C:\Users\cherrol\AppData\Roaming\Red Alert 3\Maps\`
* 游戏安装目录：`D:\game\RedAlert3\`
* 模板来源：本机社区地图 `{8p_1.5}_testing_map`（Grothen 8 人图），已复制为 `templates/8p_template.map`

## 三、常用命令

```powershell
# 一键构建默认配方（输出到 assets\<地图名>\，并打包 dist\<地图名>\[ + .zip]）
& $py tools\build_all.py

# 指定配方；构建完并安装到游戏 Maps 目录
& $py tools\build_all.py --map bagui_dadi --install

# 只跑某些阶段（terrain / passability / map / verify / preview / accept / readme / dist / wb）
& $py tools\build_all.py --only map --only verify
& $py tools\build_all.py --only accept          # 只验收当前版本产物
& $py tools\build_all.py --only dist            # 只重新打包发布件

# 同时导出并安装「新地编(World Builder)」所需文件
#   （heightmap.xyz / object.txt / 精确角度导入插件 / 官方库优化微程序）
& $py tools\build_all.py --map bagui_dadi --install --wb

# 单独跑某一环
& $py tools\terrain_gen.py --map bagui_dadi
& $py tools\map_gen.py     --map bagui_dadi
& $py tools\verify_map.py  --map bagui_dadi
& $py tools\make_preview.py --map bagui_dadi
& $py tools\make_dist.py   --map bagui_dadi [--no-zip]
```

**dist 里有什么**：只有游戏会加载的三个文件（`<地图名>.map`、`<地图名>_art.tga`、`map.str`），
放在以地图名命名的文件夹里，外加同内容的 `.zip`；预览图、说明、source/ 等一律不进 dist。
把 `dist\<地图名>\` 整个拖进 `%APPDATA%\Red Alert 3\Maps\` 即可游玩（或直接解压 zip）。

`build.ps1` / `build.cmd` 是同一条流水线的包装（`build.cmd` 不受 PowerShell 执行策略限制，推荐）：

```powershell
.\build.cmd -Map bagui_dadi -Install          # cmd 包装（推荐）
.\build.ps1 -Map bagui_dadi -Install          # 若提示脚本未签名：
                                              #   powershell -ExecutionPolicy Bypass -File .\build.ps1
```

## 四、新增一张地图

1. **建配方目录**：`maps/<new_id>/`（`<new_id>` 用 ASCII，如 `pearl_harbor`）。
2. **写 `config.py`**：直接复制 `maps/bagui_dadi/config.py` 改内容。关键字段：
   * `MAP_NAME`：地图名（= 游戏 Maps 下的文件夹名与 .map 文件名）
   * `TEMPLATE`：`templates/` 下的骨架（换玩法就换骨架）
   * `GRID / BORDER / LAND_Z / WATER_Z`：尺寸与高度基准（可玩区 = `GRID - 2*BORDER` 格，每格 `CELL` 世界单位）
   * `GEOJSON`：`maps/<new_id>/data/` 下的边界数据（任何带 `properties.name` 的 GeoJSON 多边形都行）
   * `LON0/LON1/LAT0/LAT1`：经纬度 → 地图坐标的映射范围（决定"蓝图"取景）
   * `BASES`：出生点 `(经度, 纬度, 名称, 队伍, human|ai)`
   * `RIVERS`：河流折线 + 河床高度（低于 `WATER_Z` 即形成水面与天然渡口）
   * `CITY_AMP`：各多边形（城市/地区）的山地强度，塑造"哪里高哪里平"
   * `TEX_SET / GRASS_TEXTURES`：地表纹理（**必须用本机已有地图中出现过的资源名**）
   * `WAYPOINT_LAYOUT / YARD_PLACEMENT / *_ORE / DERRICK_POS / POST_POS / DECOR_POS`：对象布局
   * `MAP_STR_*`、`PREVIEW_*`：地图文本与预览图标注
3. **放素材**：把 GeoJSON 等输入放进 `maps/<new_id>/data/`。
4. **构建**：`python tools/build_all.py --map <new_id> --install`。
5. **产物**：`assets/<MAP_NAME>/`（三个游戏文件 + 预览 + 说明 + source 自检图）。

配方里的 `TERRAIN` 字典可覆盖地形塑形参数（噪声波长/种子、山地增益、海岸与河谷影响半径、
海面深度、出生点平台尺寸等）；缺省值见 `tools/terrain_gen.py` 的 `TERRAIN_DEFAULTS`。

## 五、已知坑（写在这里，免得反复踩）

1. **压缩容器**：`.map` 磁盘形态为 `EAR\0` + uint32 解压长度 + RefPack 流；解压后必须是 `CkMp`。
   社区部分地图把大文件标志写成 `0x80`（而非 `0x90`），解析要兼容；写出统一用标准 `0x10 0xFB`。
2. **字符串表**：`uint32 数量` + 每条 `变长字符串 + uint32 id`，id 从数量降序到 1；
   属性键名也在这张表里（引用是 **uint24**）。
3. **chunk 头**：`uint32 名字索引 + uint16 版本 + uint32 数据长度 + 数据`，顺序直读到 EOF，无对齐。
   版本号随游戏变化（本机 RA3：HeightMapData=6、BlendTileData=27、SidesList=6、Script=4 …），
   解析必须按 chunk 内版本条件分支。
4. **BlendTileData 位数组语义**：`Impassability` 1=不可通行；`Buildability`、`Visibility`、
   `PassageWidths` 等以实机地图实测为准（本机实测 Buildability 全 0 = 可建造、Visibility 近全 1）。
5. **纹理块编号**：每张纹理占 `(CellSize*2)^2` 个 tile 值；格内偏移按 2×2 分组顺序
   `((y%8)//2*4+(x%8)//2)*4 + (y%2)*2 + (x%2)`（已在实机地图上验证）。
6. **纹理资源名**：`BlendTileData` 用短名（`Sand_Hawaii02`），`WorldInfo.terrainTextureStrings`
   用带前缀资源名（`TSand_Hawaii02`）配法线（`_NRM`/`_nrm` 大小写按原图照抄）。名字必须真实存在。
7. **出生位约定**：`Player_N_Start` 必须连续编号才是房间可选位；模板用
   `Player_7_Start (imported)` 这种名字让第 7/8 位**不进房间列表**，由脚本固定生成。
8. **对象 z 值**：模板中对象/waypoint 的 z 都是 0（贴地由引擎处理），改位置时保持 z=0。
9. **校验三件套**：`verify_map.py`（结构+连通）、`check_passability.py`（出生点可达性）、
   `make_preview.py`（人眼确认）——每次构建都会跑，别跳过。

## 六、当前进度与待办

* `[CHAI]八桂大地` 当前 **T1.9.8**（`maps/bagui_dadi/version.json`）。历史包只留当前版本：`python work/cleanup.py --prune-assets`。
* 玩法形态：严格分层（海底 190 / 海面 200 / 平 212 / 一层 280 / 二层 350 / 三层 420 / 高山 480）+
  16 条自动斜坡（每段层界和水岸 1–3 条，边长 28，朝向中心或资源；其余边界不可通行）+
  西 4 / 东 4 出生点（P7、P8 为固定 AI，并与东队结盟，P8 在水上）+
  每基地近矿恰好 2 个（44–62 格、矿口朝基地）+ 中路争夺矿 + 油田（14 市各 1 + 每出生点 1）+
  中部 2 哨站 / 1 车库 / 1 学院 / 1 医院 / 1 机场。
* 游戏内已安装 T1.9.8。打开这一版；更早的 T1.9.6、T1.9.7 如果还在地图目录里，是旧斜坡。
* 还没做的只有 `docs/TODO.md` 第 12 条（谷歌地形底图复核）。

