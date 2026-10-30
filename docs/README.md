# 技术文档索引

当前执行入口为 [项目首页](../README.md)，地图数量、版本快照和未完成边界见 [当前状态](SESSION_STATE.md)与[待办](TODO.md)。本地目录与 CLI id 使用真实地形名，预览标题和游戏/ZIP 文件名使用生成名，详见 [系列映射](spec/九州大地命名.md)。项目技能按 [技能索引](spec/map_skills_checklist.md) 路由。

`docs/` 按**文件性质**分区，不按主题分区：

| 目录 | 放什么 | 判定标准 |
|---|---|---|
| `spec/` | 常青规范：规则口径、验收方法、格式与流水说明 | 仍对下一次制作生效 |
| `evidence/` | 定版证据与施行记录，文件名带日期 | 只对应某一次审核或某一版结果 |
| `history/` | 已被取代的状态、待办与制作记录 | 当时成立、现在不再执行 |
| `ea-modding/` | EA 官方镜像的本地说明 | 外部数据的使用说明 |

规则归属只在[公共约束总表](spec/map_rules.md)维护：专题文档保存含义和参数口径，skills 保存执行与交接，pipeline 文档保存输入、阶段和产物，[交付验收](spec/map_acceptance.md)保存证据与判定。当前版本和历史记录不充当公共规则来源；设计优先级与构建阶段顺序分别读取专业规范与流水说明。

## 规范（`spec/`）

| 主题 | 现行规范与说明 |
|---|---|
| 公共约束关系与生效来源 | [公共约束总表](spec/map_rules.md) |
| 制作时保持层边界、水陆边界，允许添加斜坡 | [边界保护与斜坡添加](spec/preview_terrain_lock.md) |
| 对位方向与沿线布置、布矿、坡口、岸线、热点及朝向验收 | [公共地图交付验收](spec/map_acceptance.md) |
| 资料、命名与公共预览 | [系列命名](spec/九州大地命名.md)、[地形预览](spec/terrain_preview.md) |
| AXAI／NKL／SMAI 参考与斜坡、层界、水陆交接的全流程处理 | [地形交接规范](spec/terrain_transitions.md)、[交接验收](spec/map_acceptance.md#地形交接如何验) |
| 制作输入与离线恢复 | [Rust 配方](spec/rust_migration.md)、[蓝图与版本资料](spec/blueprint_versions.md)、[可复现构建](spec/reproducible_builds.md) |
| 布局搜索、两级预检、输入锁定与失败计时 | [布局规划与诊断](spec/layout_workflow.md) |
| 工程维护与缓存 | [维护工具和代码边界](spec/maintenance.md) |
| 坐标与完整成品编辑 | [通用坐标流水](spec/coordinate_pipeline.md)、[坐标公式与查询](spec/map_coordinates.md) |
| 出生点方位与两矿同面 | [公共出生点规则](spec/spawn_placement.md) |
| 斜坡优先于矿、资源落点与占地 | [优先级与冲突处理](spec/ramp_clearance.md#斜坡优先级与冲突处理)、[分级布矿](spec/resource_distribution.md)、[优先级验收](spec/map_acceptance.md#斜坡优先级如何验) |
| 预览坡道设计、可玩制作添加斜坡及既有坡体保护 | [山地坡道](spec/mountain_ramps.md)；旧增移坡／高山版本事实见[历史索引](history/README.md) |
| 材质与资源封装 | [纹理规则](spec/terrain_texture_rules.md)、[地形封装](spec/terrain_package_fix.md)、[资源封装](spec/resource_packaging_fix.md) |
| 脚本与赠送建筑 | [脚本功能分类](spec/script_functions.md)、[公共赠送规则](spec/production_placement_fix.md)、[定时进攻历史及取消](spec/periodic_attack.md) |
| 所有建筑四向朝向 | [四向规范与覆盖范围](spec/production_placement_fix.md#所有建筑的四向约束)、[逐对象／逐动作验收](spec/map_acceptance.md#建筑朝向如何验) |
| 原生格式与地编 | [地图格式](spec/ra3_map_format_spec.md)、[WorldBuilder 集成](spec/ra3_worldbuilder_integration.md) |
| 官方游戏数据与字段定义 | [EA 官方 RA3 数据镜像](ea-modding/README.md)：Xml、Schema、脚本库、官方地图源，本地 `data/ea-modding/` |
| 官方索引、最终场景与新检查 | [官方资源流水](spec/official_resource_pipeline.md)：离线目录、联合选址、XML结构、单位状态通路与严格验收／只读诊断 |

## 证据（`evidence/`）

| 主题 | 说明与机器验收 |
|---|---|
| 11 地区不合格预览修订与当前结果 | [24 处修复、44 张重绘及前后对照](evidence/preview-intrusion-repair-2026-10-30.md)、[机器验收](evidence/preview-intrusion-repair-2026-10-30.verification.json) |
| 全地区预览按最新净宽规则重新验收 | [逐区结论与窄段标记图](evidence/preview-acceptance-2026-10-30.md)、[机器汇总](evidence/preview-acceptance-2026-10-30.verification.json) |
| 全系列小水域归层与重绘结果 | [执行及验收记录](evidence/preview-water-layers.md)、[独立回读报告](evidence/preview-water-layers.verification.json) |
| 地形交接原生对照 | [AXAI／NKL／SMAI 细节图与测量证据](evidence/terrain-transitions-reference/evidence.json) |

日期目录与同名 `.md`／`.verification.json` 成组保存：说明写判断过程，`.verification.json` 写机器结果，目录放原始哈希与图片。新增一次审核就新增一组，不改写旧组。

## 历史与参考

| 主题 | 位置 |
|---|---|
| 状态与待办的历史快照 | [历史索引](history/README.md) |
| 命名迁移过程 | [命名迁移](spec/naming_migration.md) |
| 外部资料（不参与构建） | [外部参考资料索引](../reference/README.md)、[离线 RA3Wiki](../reference/ra3wiki/README.md) |

专门文档中的版本、测试数和安装结果各自对应当次记录。当前配方、实际代码和本轮授权决定制作范围；历史示例不自动成为所有地图的参数或玩法默认。生产流程不包含实机检查步骤。
