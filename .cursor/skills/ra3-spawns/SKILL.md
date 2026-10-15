---
name: ra3-spawns
description: >-
  地图出生点、同层跨度、位置分队和固定 AI 外交。改基地落点、seat_same_layer、
  verify_map 跨度或脚本分支时使用。队形和格子以当前地图配方为准，不写死某一张图。
---

# 出生点

人数、分队、谁是人类、谁是固定 AI、谁在水上，都读当前地图配方里的 `BASES` 和 `TERRAIN['base_cells']`。格子以 `base_cells` 为准。`BASES` 的经纬度是这些格的反投影，顺序对齐。

## 同层落座

陆地出生点留在同一高度层。穿过落点的同行或同列，同层连续段至少 80 格，并且离可玩边界至少 40 格。不够就在同层里换座，不把别的层刷成这一层。

水上出生点不按 80 格跨度验收，也不要求陆路连通。`verify_map.py` 只在陆地出生点跨度不足时让构建失败。

## 外交

每张图组装时都按出生位置分队，不看配方开关，也不改磁盘上的模板。

p1–p4 是一队，p5–p8 是一队。p1–p4 对应 Player_1 到 Player_4，p5、p6 对应 Player_5、Player_6，p7、p8 是两个固定 AI。队内友好，两队敌对，开局后每秒重申，房间里选的队伍盖不住。

大厅六个位子正好有四个在局内时，这四个都算西队，东队只留两个固定 AI。落在 p5 或 p6 的队伍会送到空着的 p1–p4 出生点。不是四人时，谁站在哪个位子就跟哪个位子一队，不挪位置。

固定 AI 与东队的旧分支仍在组装时对调 ScriptAction / ScriptActionFalse 的名字下标。不要改脚本名字符串（Enemy 与 Friend 长度不同）。位置分队写在这组对调之后，以位置分队为准。

p7（PlyrCivilian）每张图都是苏联欧列格，p8（PlyrCreeps）每张图都是帝国晋三。性格用 `AIPersonalityDefinition:1SovietBalanced` 和 `AIPersonalityDefinition:1JapanBalanced`。夏娜是 `3SovietAirMarshall`，贤治是 `2JapanMechaWarfare`，不占这两个位子。模板里 p8 的基地和工程师是盟军，组装时改成帝国。配方不要再写阵营、性格或建造场换型。
