# 源码功能结构

`main.rs` 是命令行入口，`lib.rs` 声明功能模块及公开接口。功能实现放在以下目录：

| 目录 | 职责 |
|---|---|
| `cli/` | 参数解析、工程定位、命令调度和标准输入输出 |
| `formats/` | 二进制读写、原生地图、ZIP、NPY 与 RefPack 编解码 |
| `pipeline/` | 配方与版本、阶段调度、验收、冻结源、增量发布、安装与导出 |
| `gameplay/` | 脚本编解码与链接、建筑航点、赠送占地/重试/朝向、可选进攻调度 |
| `terrain/` | 布局与网格、地形生成、坡形、材质、导航、占地和资源路线 |
| `resources/` | 运行时资源清单、地形图集、加载声明与脚本 XML 配套 |
| `coordinates/` | 每图坐标帧、原生对象清单、分层/边界分析、坐标报告与交互地图 |
| `render/` | 像素与文字绘制、PNG/TGA 编解码、成品预览及编号布局图 |
| `terrain_preview/` | 高程输入、平滑分层、设计预览及行政中心标注 |

## 公共实现

- `rules.rs` 从 `templates/rules/coordinate-pipeline.json` 内嵌读取出生点、坡口和资源分级的后备默认参数，避免各 `Policy::default` 重复维护数字；实际工程文件与允许的地图覆盖继续通过各领域策略加载。规则归属见 [公共约束总表](../docs/map_rules.md)。
- `formats/npy.rs` 统一读取和写入 NumPy 数组，供新图地形与设计预览使用。
- `terrain/grid.rs` 集中网格翻转、邻接遍历、距离场、梯度和双线性采样。矩形遍历显式接收宽高，方形接口复用同一实现。
- `terrain/layout.rs` 管理布局模型及发布版布局读取；读取冻结制作源仍由 `pipeline/frozen_source.rs` 校验源包与 CRC。
- `terrain/layers.rs` 按原字节同步选定蓝图到地图根 `layers.npz`，供构建、发布与采用制作源共用；历史输入放在 `maps/<id>/histories/<版本>/`。
- `gameplay/scripts/arguments.rs` 统一动作/条件叶子编码、标量参数和原生数值偏移；各玩法保留自己的命令签名与触发条件检查。
- `gameplay/placement.rs` 统一生产类型、可退让装饰身份、所属基地接入与最终主备占地叠加检查；`decoration.rs` 实际迁移受阻的合格装饰，`gift_clearance.rs` 共享矿盘／旋转矿场／坡口预留，供赠送及原生生产选址使用。规划与验收边界见[统一选址](../docs/production_placement_fix.md#布局优先级与统一选址)。
- `pipeline/build/` 分离阶段/事务、地图组装、回读验收、成品分发和落点证明；新图的通行与组装阶段共享同一次地表计算，组装修改布局后重新加载。对象落点与分类清单在全部落点修复后一起同步，航点报告记录最终原生坐标。
- `pipeline/acceptance.rs` 维护独立交付记录、候选身份和待审归档；分发、安装、增量发布及采用制作源共用门禁，不从自动几何结果生成独立通过结论。记录格式与证据边界见[结构化独立验收](../docs/map_acceptance.md#结构化独立验收门禁)。
- `terrain/spatial.rs` 的近矿审计复用生效出生策略；赠送选址与生产占地校验共享 `production_spawns::PlacementPolicy`。冻结空间缓存由 `pipeline/coordinate_pipeline.rs` 绑定当前原生内容、布局、生效策略、基准及算法版本，不继承历史问题。
- `terrain/texture_polish.rs` 区分绘制阶段报告和最终封装记录，后者保存清理后的材质编号映射。完整构建将常规预览交给后续预览阶段，单独组装和成品编辑仍输出完整资料。
- `resources/` 分离资源识别、图集同步、原生地形查找表和 XML 输出。资源生成与刷新共享脚本列表写入，调用方保留各自的完整性门禁。
- `resources/official.rs` 一次读取固定版本 EA 离线索引；`xml_structure.rs` 检查 GameMap/Atlas 结构子集。显式材质三元组覆盖官方逻辑名，冻结图集保留原声明。
- `pipeline/scene.rs` 绑定最终原生对象、XML、覆盖内容与生效策略，统一输出官方占地/XML/单位状态通路资料。`official_placement` 复用最终合法朝向检查地基与出口，`mobility` 只作能力/状态分开的静态诊断；兼容边界见[官方资源流水](../docs/official_resource_pipeline.md)。
- `coordinates/` 分离栅格分析、对象点清单和导出；`terrain_preview/` 分离分层算法与绘制；`render/image.rs` 负责图像基础能力。

## 接口与变更约束

模块按功能路径引用，例如 `ra3maps::formats::map`、`ra3maps::gameplay::scripts` 和 `ra3maps::pipeline::build`。`lib.rs` 声明功能域与通用的 `Error`、`Result`、`err`；源码、测试和工具统一使用功能域路径。

跨领域能力直接从所属模块引用：NPY 使用 `formats::npy`，网格邻接使用 `terrain::grid`。`terrain::Layout`、`gameplay::scripts::parse_nodes` 等接口由对应领域集中导出。

合并算法需保留数据格式、浮点运算顺序、遍历顺序、错误检查和作用范围。水陆阈值、地形保护及不同版本原生格式的差异由对应模块明确处理。纯结构重构保持地图字节和版本；通过干净环境的格式、严格 Clippy 与完整回归核对。
