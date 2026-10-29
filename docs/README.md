# 技术文档索引

当前执行入口为 [项目首页](../README.md)，地图数量、版本快照和未完成边界见 [当前状态](SESSION_STATE.md)与[待办](TODO.md)。本地目录与 CLI id 使用真实地形名，预览标题和游戏/ZIP 文件名使用生成名，详见 [系列映射](九州大地命名.md)。项目技能按 [技能索引](map_skills_checklist.md) 路由；历史记录单独见 [历史索引](history/README.md)。

| 主题 | 现行规范与说明 |
|---|---|
| 资料、命名与公共预览 | [系列命名](九州大地命名.md)、[地形预览](terrain_preview.md) |
| 制作输入与离线恢复 | [Rust 配方](rust_migration.md)、[蓝图与版本资料](blueprint_versions.md)、[可复现构建](reproducible_builds.md) |
| 工程维护与缓存 | [维护工具和代码边界](maintenance.md) |
| 坐标与完整成品编辑 | [通用坐标流水](coordinate_pipeline.md)、[坐标公式与查询](map_coordinates.md) |
| 出生点方位与两矿同面 | [公共出生点规则](spawn_placement.md) |
| 资源落点与占地 | [公共坡口避让](ramp_clearance.md)、[公共分级布矿规则](resource_distribution.md) |
| 斜坡与高山 | [通用山地斜坡](mountain_ramps.md)、[移动记录](ramp_relocation.md)、[新增记录](ramp_additions.md)、[高山记录](impassable_mountains.md) |
| 材质与资源封装 | [纹理规则](terrain_texture_rules.md)、[地形封装](terrain_package_fix.md)、[资源封装](resource_packaging_fix.md) |
| 脚本与赠送建筑 | [脚本功能分类](script_functions.md)、[公共赠送规则](production_placement_fix.md)、[定时进攻历史及取消](periodic_attack.md) |
| 原生格式与地编 | [地图格式](ra3_map_format_spec.md)、[WorldBuilder 集成](ra3_worldbuilder_integration.md) |
| 历史与参考资料 | [版本/会话快照](history/README.md)、[命名迁移](naming_migration.md)、[离线 RA3Wiki](ra3wiki/README.md) |

专门文档中的版本、测试数和安装结果各自对应当次记录。当前配方、实际代码和本轮授权决定制作范围；历史示例不自动成为所有地图的参数或玩法默认。生产流程不包含实机检查步骤。
