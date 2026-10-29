# 项目技能索引

五份项目技能按当前原生 Rust 制作路径组织，不维护另一套历史规则。先确定任务范围，再读取对应技能与其链接的专门文档。

| 任务 | 技能 | 相关规范 |
|---|---|---|
| 真实资料、三张基础预览、可选中心图、新图规划 | [ra3-authoring](../.agents/skills/ra3-authoring/SKILL.md) | [公共预览](terrain_preview.md)、[命名](九州大地命名.md) |
| 分层、轴向、水陆、坡形、纹理和通行 | [ra3-terrain](../.agents/skills/ra3-terrain/SKILL.md) | [地图坐标](map_coordinates.md)、[山地斜坡](mountain_ramps.md)、[纹理规则](terrain_texture_rules.md) |
| 出生点、固定 AI 归属/外交、脚本主备航点 | [ra3-spawns](../.agents/skills/ra3-spawns/SKILL.md) | [出生点方位、两矿同面与水陆均衡](spawn_placement.md)、[赠送建筑规则](production_placement_fix.md) |
| 矿、油井、中立建筑、完整矿场占地与坡口避让 | [ra3-resources](../.agents/skills/ra3-resources/SKILL.md) | [公共坡口规则](ramp_clearance.md)、[公共分级布矿规则](resource_distribution.md) |
| 构建、验收、版本/回滚、增量编辑、冻结源和发布 | [ra3-pipeline](../.agents/skills/ra3-pipeline/SKILL.md) | [坐标流水](coordinate_pipeline.md)、[可复现构建](reproducible_builds.md) |

所有地图先建立自己的 Frame，资源由该帧计算位置；坡口检查包含完整宽度、双向端口区和资源/建筑完整预留占地。新图与冻结成品使用不同来源；旧问题留档，本轮新增/恶化项拦截，不自动挪动旧图。流程不包含实机检查步骤。

入口与能力边界见 [项目首页](../README.md)、[当前状态](SESSION_STATE.md)、[待办](TODO.md)；旧流程和会话文字保存在 [历史索引](history/README.md)。
