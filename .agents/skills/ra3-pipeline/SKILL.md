---
name: ra3-pipeline
description: 原生 Rust 地图构建、成品增量编辑、验收、冻结制作源、版本、发布与安装。
---

# 原生制作与发布流水

按[公共约束总表](../../../docs/spec/map_rules.md)确定规则来源，按[技能分工](../../../docs/spec/map_skills_checklist.md)收集各专业的结果。本技能负责构建、交付汇总、版本和完整来源，不另设设计阈值或重复专业验收。

遵守[边界保护与斜坡添加](../../../docs/spec/preview_terrain_lock.md)：可玩构建保留选定预览或冻结源的层边界与水陆边界，允许添加斜坡及必要的局部坡面高度、通行变化；层边界按设计分层标签／掩码核对，不以合法斜坡的局部高度变化判为边界变化。既有坡道仍受保护，不借新增斜坡搬移、删改既有坡或铺平台。布局无解则记录冲突，确需改边界时返回预览阶段。只改流程时不检查或改动已完成的地图。

## 输入与编辑入口

新图读取本图声明式配方、`LAYERS`、`LAYOUT` 和 Frame；预览缓存先整理为构建输入。已有成品使用 `COORDINATE_PIPELINE` 引用的完整原生 ZIP、布局、基准和 CRC，保留本轮范围外内容及对象／脚本身份。输入格式与离线恢复见[Rust 配方](../../../docs/spec/rust_migration.md)、[坐标流水](../../../docs/spec/coordinate_pipeline.md)和[可复现构建](../../../docs/spec/reproducible_builds.md)。

```sh
cargo run --release --locked -- build --map <id>
cargo run --release --locked -- build --map <id> --bump --note '本轮修改'
cargo run --release --locked -- accept --map <id>
```

阶段顺序以 `src/pipeline/build/mod.rs::STAGES` 为准；`--only`／`--skip` 不补前置产物。坐标编辑修改配置指定布局；材质整理用 `polish-textures` 或 `polish-ramp-surfaces`，封装修复用相应 `repair-*`，这些处理保持几何。普通 `repack` 仅严格验收和重封装；新增斜坡使用明确记录范围的构建输入或编辑入口，核对实际差异仅含该坡必要的局部变化，禁止顺带重算其他地形或改变边界、既有坡位。

赠送局部受阻使用 `repair-gifts <源ZIP> <计划JSON> --map <id> --bump`，源 CRC、航点策略及主体区域格式见[受阻修复与重选](../../../docs/spec/production_placement_fix.md#已有成品的受阻修复与重选)。将计划、完整冲突和局部／基地候选比较一并交接审核；缺少记录时保留待审候选，不借整体生产重排扩大局部任务。

## 验收与专业交接

按[公共交付验收](../../../docs/spec/map_acceptance.md)逐项汇总适用范围、状态和同版证据；读取实际自动报告和独立记录，不从 CLI 成功推定完整通过。自动 `passed`、`raw_passed` 均须通过，完整来源不提供历史问题豁免。

交付入口按[结构化独立验收](../../../docs/spec/map_acceptance.md#结构化独立验收门禁)核对候选文件、完整布局、生效参数及逐项审核证据。缺记录时先保留待审候选，完成专业判断后重试；不得把旧报告或自动结果直接转成独立通过。`prepare-delivery` 可为当前候选生成待核验清单。

核对地形／资源交接的[斜坡优先级验收](../../../docs/spec/map_acceptance.md#斜坡优先级如何验)：全部有效坡道及完整上下引道优先于矿，冲突须有矿位退让及关联复验记录。为保矿削改、缩窄或挪动合理坡道的候选，即使自动通过也不能交付。路线变更再交接[沿线布置验收](../../../docs/spec/map_acceptance.md#对位进攻方向与沿线布置如何验)；其他专业结果按验收索引与技能分工收齐，流水不重新维护完整检查清单。

涉及坡面、层界、岸线或纹理时，按[地形交接规范](../../../docs/spec/terrain_transitions.md)收齐[交接验收](../../../docs/spec/map_acceptance.md#地形交接如何验)的同版几何、材质、导航、视觉及交汇处证据，并关联现有坡口、岸线和通行记录。公共工具尚未完整自动覆盖；不能从构建、纹理整理或封装成功推断衔接通过，冻结与发布前保留未闭环项的失败／待核验状态。

跨专业清单须合并去重并核对覆盖完整，例如[建筑朝向](../../../docs/spec/map_acceptance.md#建筑朝向如何验)合并出生／资源技能的全部建筑、`OreNode` 及脚本动作。各专业复用同一输入身份与共享测量；沿用旧证据须按验收文件证明相关输入未变。

## 定稿、冻结与安装

核对实际差异、完整游戏文件、同版 `source/layout.json` 及全部适用验收。失败或待核验项闭环后，按[版本资料](../../../docs/spec/blueprint_versions.md)冻结当前完整来源与选定蓝图镜像：

```sh
cargo run --release --locked -- adopt-coordinate-source --map <id>
cargo run --release --locked -- build --map <id>
```

增量发布不自动采用来源或完成坐标／WB 导出，采用新基准不能掩盖失败。内容变化递增版本；仅更新规范或工具且地图内容未变时不递增。提交前失败按事务恢复版本与根镜像，只清理本轮新产物；成品删除范围按用户要求执行。

制作资料放 `assets/<id>/<发布名>/`，游戏成品放 `dist/<id>/<发布名>/` 和同名 ZIP。安装仅按已有授权使用 `--install` 写入 `APPDATA/Red Alert 3/Maps/<发布名>/`；安装失败保留已提交本地成果并报告。WB 默认导出资料，复制地编目录按已授权目标执行。生产流程不包含启动游戏或 GUI 检查。
