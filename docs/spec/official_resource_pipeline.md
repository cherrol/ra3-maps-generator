# 官方资源索引与最终场景验收

流水使用固定版本的 EA 资源事实描述实体，项目公共参数继续决定布矿、四向、坡口净距及额外预留。新检查覆盖 XML 结构子集、官方建筑几何和显式生产出口；单位状态通路是独立诊断，不替代公共交付验收。

## 索引与复现

`templates/resources/ea-catalog.json` 从本地 EA 镜像离线提取，固定上游 revision `3aad8e412961ab543192ae70f8503dc8e178e095`。当前索引包括 406 种材质、555 个限定来源对象和 121 个移动定义；绑定提取器版本、提取器源文件及实际输入 SHA-256。`resources::official` 用 `OnceLock` 读取内嵌 JSON，普通 Rust 构建不读取镜像、不联网；索引只由 Rust 作者工具离线生成。

```sh
cargo test --locked --example import-ea-catalog
cargo run --release --locked --example import-ea-catalog -- --check
# 明确更新索引时执行；先核对 revision 和来源差异
cargo run --release --locked --example import-ea-catalog
```

`--check` 逐字节比对已入库索引；提取器为 `xtask/import_ea_catalog.rs`，其自身与 `src/formats/sha256.rs` 的摘要一并写入绑定，改动提取器后必须重新生成并审核差异。

提取器跟随官方静态入口和已验证的对象来源，处理作用域继承、XInclude 片段及已使用的 xpointer、KindOf/Surfaces 增减和相关 schema 默认值。不会扫描全目录后任取一个重名定义。全局预加载身份继续来自已验证的 `WBStatic_12` 成员表，文件存在不等于全局预加载。无法唯一解析的对象、表达式和行为保留诊断；索引是本项目需要的元数据子集，不是 EA 资产编译器的替代实现。

同名对象的所有候选若具有完全一致、完整解析的 KindOf，仅导出分类共识及全部来源。目前两个信标据此确认为无碰撞标记，避免误阻挡选址；完整对象和几何仍保持未解析，不任选来源。

`TEX_SET` 可以使用官方逻辑名字符串；已有 `[逻辑名, diffuse, normal]` 三元组仍保留为显式覆盖。索引存在某材质不会把未使用材质加入地图。冻结图集的实际声明继续优先，49 种上限沿用现有项目门禁，不归因于 XSD。

## 最终场景和坐标

资源和脚本落定后，以同一份最终 `MapFile` 生成 `source/final-scene.json`：保留原生对象顺序、实际坐标、朝向、来源与未覆盖对象。随后各项检查只读这份原生内容，封装与导出继续遵循原有事务。

格坐标、世界坐标和对象局部坐标统一使用 `coordinates::Frame`。官方局部偏移先按实际原生弧度旋转，再经 `Frame.local_to_grid` 换算，单位由本图格长决定。坐标声明用于本图和离线版本资料，内容相同时不再重复写盘；不另建一套官方坐标帧。非零旋转锚点、未支持的建筑实例缩放等情况明确记为 pending。

## 自动检查范围

| 检查 | 实现与适用范围 |
|---|---|
| XML 结构 | 检查 EA 命名空间、GameMap 对象分组顺序、单例、位置/角度和图集必要字段；不是完整 XSD 或游戏运行验证。航点唯一性作为项目诊断单列，由原生对象/脚本门禁检查，不冒充 XSD 唯一性约束。所有生产 XML 按当前分组规则严格检查；新增航点插入正确分组，保留无关原生内容不能成为结构错误的豁免。 |
| 官方几何 | 独立处理 PlacementShape、多块碰撞 Shape 和原有项目预留；放置形状不被用作碰撞或寻路形状。候选选址和最终动作复用同一合法朝向算法，覆盖主备和显式条件分支。 |
| 生产出口 | 旋转实际 UnitCreatePoint/NaturalRallyPoint，检查静态扫掠区；未知出口、NoExitPath、实例变体分别报告，不从骨骼名编造坐标。 |
| 矿场证据 | 记录 OreNode 建议偏移、限制区和三阵营矿场几何，同时保留项目矿盘/规划矩形。不是矿车骨骼停靠或采矿效率模拟。 |
| 单位通路 | 以原生 StandingWaterAreas、水位和地形计算水深，分别分析代表单位每个 LocomotorSet 条件。普通和出厂状态不自动串联；未知水域格式、河流、障碍或特殊运动能力标为 unsupported/partial。 |

所有可玩地图的构建、坐标编辑、完整来源重建、原生增量编辑、重新封装、采用源、分发及安装均严格执行适用的官方建筑检查和 XML 结构检查。完整来源、原生字节未变、旧发布记录、`RULESET` 或历史参考均不能切换为宽松生产模式。

```json
{
  "OFFICIAL_PLACEMENT": {"mode": "strict"},
  "XML_STRUCTURE": {"mode": "strict"}
}
```

生产模式缺省为 `strict`；显式 `diagnostic` 只用于只读检查，不能授权生产、验收或发布。未知项不记为合格。每个包的实际 `overrides.xml` 内容、`map.xml` 内联资产及非 SDK 本地 Include 参与检查和缓存绑定；尚未解析的局部资产覆盖使相关官方几何精度待核验，严格模式拒绝用原版索引代替覆盖后的事实。生产修复在提交新版本前同样执行该门禁。`DATA:`／`ART:` 引用按基础 SDK 来源处理，不宣称递归解析外部资产编译结果。

采用来源与发布按[公共交付验收](map_acceptance.md)汇总本报告和其他适用证据；临时填写 `COORDINATE_PIPELINE` 不能为尚未通过的候选建立豁免。发布证明必须匹配实际地图、XML、覆盖内容、版本及当前规则，不能继承原默认诊断记录。最终场景检查不判断斜坡与矿的设计取舍，该项另按[斜坡优先级验收](map_acceptance.md#斜坡优先级如何验)留证。

报告位于本版 assets：`scene-report.json`、`official-placement.json`、`xml-structure.json`、`unit-routes.json` 和 `source/final-scene.json`。`passed` 表示生效自动门禁通过；`raw_passed`、pending、覆盖范围和独立交付验收应分别阅读。缓存绑定实际原生/XML/覆盖内容、生效配置、公共策略和官方索引身份，同时检查导出文件校验值。

最终生产审计覆盖全部适用原生建筑及显式创建动作，生产建筑修复也执行相同范围；配置 `scope` 不能缩小检查范围。官方占地和最终场景的 `passed`、`raw_passed` 均须为真，不能以局部检查结果授权发布。

```sh
cargo run --locked -- audit-scene --map <id>
cargo run --locked -- check-xml <map.xml>
```

`audit-scene` 只生成检查资料，不重建或发布地图；严格模式失败会保留失败报告并返回错误。`check-xml` 只读输入并输出结构子集报告。

## 官方地图语料

普通测试读取 `tests/fixtures/official-map-fragments.zip` 中四张官方图的 SHA-256 绑定片段，覆盖不同边框、440×640 矩形、环境区块版本和未知区块保留，不要求本机存在整份镜像。片段明确区分原始字节与为最小测试构造的数据，不把小夹具称为完整官方地图。

```sh
cargo run --release --locked --example verify-official-maps -- --check-fixture
# 显式验证本机完整镜像；缺失镜像会报告 unsupported 并失败
cargo run --release --locked --example verify-official-maps -- --verify-full
```

`verify-official-maps` 自带第二套 MSB 容器解码器，只通过 CLI 做 `pack`/`unpack` 往返，不复用 `formats::map`，因此编解码回归仍由独立实现判定。

官方片段仅用于格式解析和编解码回归，不是可发布地图，也不提供本项目八席、48矿或严格验收规则的放行入口。矩形导航按实际宽高遍历，不改变项目新图的正方形制作规范。
