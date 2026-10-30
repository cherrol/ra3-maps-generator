# 九州大地·沃野奔流 T0.1.2 验收

本版为材质专项：候选的高度图、全部通行／建造位图、对象与脚本逐字节等于 T0.1.1，26 个原生区块中只有 `BlendTileData` 变化。
因此几何与玩法类项目沿用 T0.1.1 的已验收结论，证据是同版逐字节对照；坡口项目另附材质重绘与坡肩对比。

## 材质改动

| 项目 | 改前 | 改后 |
|---|---:|---:|
| 重绘格数 | 0 | 21,124 |
| Rock_Yucatan01 | 11,583 | 8,799 |
| Rock_Yucatan03 | 756 | 1,057 |
| Dirt_Yucatan06 | 6,808 | 603 |
| Gravel_Yucatan01 | 6,438 | 648 |
| 混合描述 | 2,888 | 2,604 |
| 材质边界 | 32,820 | 27,915 |
| 无混合硬缝 | 646 | 551 |

判据：陡面按实测坡度（≥8 高度单位/格）识别，含 4–6 格坡肩；坡脚土带 1 格；岸线 4 格内不落岩面。

## 13 项

### 对位进攻方向与沿线布置（`attack_routes`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。21 坡坡体与高度未改，主次通路几何不变。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 建筑朝向（含矿脉）（`building_angles`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。对象列表逐字节相同，含矿脉朝向。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 赠送建筑避开伸入区域（`gift_intrusion_exclusion`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。赠送对象与位图未改。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 热点双方实际路程（`hotspot_distances`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。高度相同，双方实际路程不变。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 中立建筑选址（`neutral_building_siting`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。中立建筑对象逐字节相同。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 布矿（`ore_distribution`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。48 矿对象逐字节相同。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 赠送生产建筑靠近基地（`production_base_proximity`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。赠送生产建筑落点与位图未改。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 坡口（`ramp_clearance`）

坡面材质改为按实测坡度识别陡面：坡度 ≥8（每格高度单位）即岩面，含 4–6 格坡肩；坡脚土带收窄为 1 格，去掉沿崖线的宽土带。共重绘 21124 格，材质边界 32820→27915、无混合硬缝 646→551。坡口高度、宽度、坡形与引道未改；仅坡肩与坡脚材质调整。

证据：material-change.json, candidate-files.json, ramp-shoulder-materials.png, ../T0.1.1/delivery-acceptance.json

### 斜坡优先于矿（`ramp_priority`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。矿与坡体几何未改，坡优先结论不变。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 交战区与资源通行便利（`resource_access`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。通行位图与高度相同，资源接入不变。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 交战路线宽度与伸入地形（`route_width_intrusions`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。净宽由高度与通行位图决定，未改。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

### 整圈岸线（`shore_perimeter`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。水陆几何与沙带规则保留（岸线 4 格内不落岩面）。

证据：material-change.json, candidate-files.json, before-after-materials.png, ../T0.1.1/delivery-acceptance.json

### 出生座位布局（`spawn_layout`）

候选高度图、全部通行／建造等玩法位图、对象列表与脚本逐字节等于 T0.1.1（高度 crc32 3773833044、位图 706440 字节 crc32 1593470479、26 个区块中仅 BlendTileData 变化）；本版只改地表材质与混合记录，因此本项几何／玩法结论沿用 T0.1.1 已验收结论。

证据：material-change.json, candidate-files.json, ../T0.1.1/delivery-acceptance.json

## 发布与采用

- 本版 5 个文件在 assets、dist、ZIP 与 `authoring/source.zip` 逐字节相同；`current_frozen_map_rebuild_preserves_bytes_and_passes_complete_validation` 通过，说明按新源完整复构仍是同一组字节。
- 已执行 `adopt-coordinate-source --map 河南`，冻结几何源改为 T0.1.2；坡口几何、出生组、资源接入、赠送与最终接入校验随采用流程重新运行并通过。
- 全量测试 328 项通过、0 失败，Clippy 无警告。

## 未做的验证

- 未启动游戏，未做实机近景确认；文中只报告材质分配、混合记录与字节对照。
- 游戏目录在当前环境为只读挂载，本版未安装。
- 斜向层界的一格栅格台阶属于高度场形状，本版未改高度，未处理。
- 本版没有重新挖掘或重建地形；采用新源后的完整复构与发布字节相同，不等于重新设计几何。
