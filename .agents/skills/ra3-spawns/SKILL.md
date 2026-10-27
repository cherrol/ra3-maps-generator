---
name: ra3-spawns
description: >-
  地图出生点、同层跨度、位置分队和固定 AI 外交。改基地落点、seat_same_layer、
  verify_map 跨度或脚本分支时使用。队形和格子以当前地图配方为准，不写死某一张图。
---

> **Rust 迁移适配（优先于下文旧工具引用）**：现行配方是 `config.json` 与 `layout.json`，构建使用 `cargo run --release -- build --map <id>`，验收使用 `cargo run --release -- accept --map <id>`。下文旧自动搜索函数、Python 开关和历史预览流程只描述原业务规则，不是可执行入口。出生、斜坡、资源、装饰由布局显式定稿；修改坐标需同步对象和验收集合。玩法由 `RULESET` 引用 `templates/rules/`。操作方式以 `docs/rust_migration.md` 为准。


# 出生点

人数、分队、谁是人类、谁是固定 AI、谁在水上，都读当前地图配方里的 `BASES` 和 `TERRAIN['base_cells']`。格子以 `base_cells` 为准。`BASES` 的经纬度是这些格的反投影，顺序对齐。

## 同层落座

陆地出生点留在同一高度层。穿过落点的同行或同列，同层连续段至少 80 格，并且离可玩边界至少 40 格。不够就在同层里换座，不把别的层刷成这一层。

水上出生点不按 80 格跨度验收，也不要求陆路连通。`verify_map.py` 只在陆地出生点跨度不足时让构建失败。

## 外交

以配方 `RULESET` 选择的脚本为准，构建只链接模板，不额外注入外交或落座规则。当前通用模板为 `common-T0.5.1`，包含 224 条脚本；不含电脑现金显示、电脑主动进攻、按位置分队和四人落座。固定 AI 初始化外交保留源脚本行为。

p1–p6 对应 Player_1 到 Player_6，p7 为 PlyrCivilian（苏联欧列格），p8 为 PlyrCreeps（帝国晋三）。性格和初始化以模板为准，建造场及工程师类型在布局中定稿，不在组装时另做换型。

通用模板的 `spawn_waypoints` 声明 31 个超武、电厂航点。Rust 按本图出生点寻找平坦陆地并检查占地、障碍和间距，写入 `script_waypoints.json`。调整出生点需同时检查这些航点、`dual-*` 产能点和固定 AI 对象。水上基地的新增脚本建筑也落在附近陆地，兼容各阵营。找不到安全空间时应调整该图布局，不放宽通用验收。

赠送重工的完整占地必须在平坦陆地，船厂的完整占地必须在水域。`production_spawns` 从实际创建动作解析类型及航点，并覆盖预置重工、船厂；构建后修正无效位置，成品回读再次验收。默认搜索 80 格，沿用最近出生位的 `SCRIPT_SPAWN_RADIUS_BY_SEAT`，可由 `PRODUCTION_SPAWN_RADIUS` 显式配置。不可回退到错误水陆或障碍区域。报告为 `production-spawns.json`。已有局部优化成品使用 `repair-production --map <id> --bump <源ZIP>`，只移动错误落点并同步 XML，保留地形、脚本和资源身份。
