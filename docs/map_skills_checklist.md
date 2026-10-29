# 项目技能索引

五份项目技能按当前原生 Rust 制作路径组织。规则归属、参数来源、作用范围与自动覆盖状态统一见 [公共约束总表](map_rules.md)。技能只负责工作流和规范路由，不维护另一套阈值、方位列表或历史版本规则。

| 任务 | 技能 | 相关规范 |
|---|---|---|
| 真实资料、三张基础预览、可选中心图、新图规划 | [ra3-authoring](../.agents/skills/ra3-authoring/SKILL.md) | [公共预览](terrain_preview.md)、[命名](九州大地命名.md) |
| 分层、轴向、水陆、坡形、纹理和通行 | [ra3-terrain](../.agents/skills/ra3-terrain/SKILL.md) | [地图坐标](map_coordinates.md)、[山地斜坡](mountain_ramps.md)、[纹理规则](terrain_texture_rules.md) |
| 出生点、固定 AI 归属/外交、脚本主备航点与赠送产能选址 | [ra3-spawns](../.agents/skills/ra3-spawns/SKILL.md) | [出生点方位、两矿同面与水陆均衡](spawn_placement.md)、[赠送建筑规则](production_placement_fix.md)、[所属基地选址](production_placement_fix.md#赠送生产建筑靠近所属基地) |
| 矿、油井、中立建筑、完整矿场占地与坡口避让 | [ra3-resources](../.agents/skills/ra3-resources/SKILL.md) | [公共坡口规则](ramp_clearance.md)、[公共分级布矿规则](resource_distribution.md) |
| 构建、验收、版本/回滚、增量编辑、冻结源和发布 | [ra3-pipeline](../.agents/skills/ra3-pipeline/SKILL.md) | [坐标流水](coordinate_pipeline.md)、[可复现构建](reproducible_builds.md) |

所有地图先建立自己的 Frame，资源由该帧计算位置；坡口检查包含完整宽度、双向端口区和资源/建筑完整预留占地。新图与冻结成品使用不同来源；旧问题留档，本轮新增/恶化项拦截，不自动挪动旧图。流程不包含实机检查步骤。

五份技能共同执行[公共地图交付验收](map_acceptance.md)：布矿、坡口、交战区与资源通行便利、整圈岸线、热点双方实际路程、建筑朝向逐项记录适用范围、状态及同版证据。资源技能负责完整分级、双方路程及资源／中立建筑朝向；地形技能负责坡形、坡口、通行布局和全周岸线；出生点技能联动重布、预置和脚本建筑影响，并提交[赠送产能靠近基地的验收](map_acceptance.md#赠送生产建筑靠近基地如何验)；制作与流水技能在定稿前汇总全部建筑和矿脉四向清单及其他必检结果。建筑朝向验收统一遵守[四向规范](production_placement_fix.md#所有建筑的四向约束)，包含全部 `OreNode`，不局限于赠送建筑，不因矿脉属于资源而跳过。公共 `ore_facing` 门禁与独立验收共同执行；自动检查未覆盖的项必须独立测量，缺证据记待核验，不能据 CLI 通过直接宣称完整验收通过。

斜坡执行[数量与通行布局规则](mountain_ramps.md#斜坡数量与通行布局)：数量不限，按玩家通往交战区及矿、油井、医院等全部中立资源的需要增设或移动；按[通行便利验收](map_acceptance.md#交战区与资源通行便利如何验)核对实际路径、绕行和瓶颈，静态可达或某个坡数不能代替便利性结论。

制作、地形与流水技能同时落实[小水域单层包裹](terrain_preview.md#小水域的单层包裹)：按调整前各层接岸长度选最多接触层，局部归并全周岸边陆地；验收保留接触统计、归属和前后分层，检查整圈只接触归属层，再核对连续水陆过渡。该项尚无公共自动门禁，须独立留证。

五份技能共同执行[交战路线与伸入地形整理](terrain_preview.md#交战路线与伸入地形整理)：交战段净宽至少 10 格，不足适当拓宽；细长伸入的层、陆地、水域净宽至少 5 格，不足就近归并；全部赠送主备及条件分支的完整占地避开伸入区域。参数统一读取 `coordinate-pipeline.json#/terrain_shape`，技能不另存数值；制作／地形负责范围、拓宽与归并，出生／资源联动完整占地和赠送排除，流水汇总[独立验收](map_acceptance.md#交战路线宽度与伸入地形如何验)。当前三项尚无完整公共自动门禁，须保留同版测量及前后证据。

入口与能力边界见 [项目首页](../README.md)、[当前状态](SESSION_STATE.md)、[待办](TODO.md)；旧流程和会话文字保存在 [历史索引](history/README.md)。
