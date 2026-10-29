# 官方资源索引与最终场景验收

流水使用固定版本的 EA 资源事实描述实体，项目公共参数继续决定布矿、四向、坡口净距及额外预留。新检查覆盖 XML 结构子集、官方建筑几何和显式生产出口；单位状态通路是独立诊断，不替代公共交付验收。

## 索引与复现

`templates/resources/ea-catalog.json` 从本地 EA 镜像离线提取，固定上游 revision `3aad8e412961ab543192ae70f8503dc8e178e095`。当前索引包括 406 种材质、555 个限定来源对象和 121 个移动定义；绑定提取器版本及实际输入 SHA-256。`resources::official` 用 `OnceLock` 读取内嵌 JSON，普通 Rust 构建不调用 Python、不扫描镜像、不联网。

```sh
python3 tools/import_ea_catalog.py --test
python3 tools/import_ea_catalog.py --check
# 明确更新索引时执行；先核对 revision 和来源差异
python3 tools/import_ea_catalog.py
```

提取器跟随官方静态入口和已验证的对象来源，处理作用域继承、XInclude 片段及已使用的 xpointer、KindOf/Surfaces 增减和相关 schema 默认值。不会扫描全目录后任取一个重名定义。全局预加载身份继续来自已验证的 `WBStatic_12` 成员表，文件存在不等于全局预加载。无法唯一解析的对象、表达式和行为保留诊断；索引是本项目需要的元数据子集，不是 EA 资产编译器的替代实现。

同名对象的所有候选若具有完全一致、完整解析的 KindOf，仅导出分类共识及全部来源。目前两个信标据此确认为无碰撞标记，避免误阻挡选址；完整对象和几何仍保持未解析，不任选来源。

`TEX_SET` 可以使用官方逻辑名字符串；已有 `[逻辑名, diffuse, normal]` 三元组仍保留为显式覆盖。索引存在某材质不会把未使用材质加入地图。冻结图集的实际声明继续优先，49 种上限沿用现有项目门禁，不归因于 XSD。

## 最终场景和坐标

资源和脚本落定后，以同一份最终 `MapFile` 生成 `source/final-scene.json`：保留原生对象顺序、实际坐标、朝向、来源与未覆盖对象。随后各项检查只读这份原生内容，封装与导出继续遵循原有事务。

格坐标、世界坐标和对象局部坐标统一使用 `coordinates::Frame`。官方局部偏移先按实际原生弧度旋转，再经 `Frame.local_to_grid` 换算，单位由本图格长决定。坐标声明用于本图和离线版本资料，内容相同时不再重复写盘；不另建一套官方坐标帧。非零旋转锚点、未支持的建筑实例缩放等情况明确记为 pending。

## 自动检查范围

| 检查 | 实现与适用范围 |
|---|---|
| XML 结构 | 检查 EA 命名空间、GameMap 对象分组顺序、单例、位置/角度和图集必要字段；不是完整 XSD 或游戏运行验证。航点唯一性作为项目诊断单列，由原生对象/脚本门禁检查，不冒充 XSD 唯一性约束。新生成 XML 按分组输出；冻结来源只诊断，新增航点插入正确分组，不全量改排旧 XML。 |
| 官方几何 | 独立处理 PlacementShape、多块碰撞 Shape 和原有项目预留；放置形状不被用作碰撞或寻路形状。候选选址和最终动作复用同一合法朝向算法，覆盖主备和显式条件分支。 |
| 生产出口 | 旋转实际 UnitCreatePoint/NaturalRallyPoint，检查静态扫掠区；未知出口、NoExitPath、实例变体分别报告，不从骨骼名编造坐标。 |
| 矿场证据 | 记录 OreNode 建议偏移、限制区和三阵营矿场几何，同时保留项目矿盘/规划矩形。不是矿车骨骼停靠或采矿效率模拟。 |
| 单位通路 | 以原生 StandingWaterAreas、水位和地形计算水深，分别分析代表单位每个 LocomotorSet 条件。普通和出厂状态不自动串联；未知水域格式、河流、障碍或特殊运动能力标为 unsupported/partial。 |

新图默认严格执行适用的官方建筑检查和 XML 结构检查。冻结成品、legacy 配方及不可变历史参考默认诊断，保留既有内容和原门禁。显式 `repair-production` 严格检查本轮工厂、船厂及赠送动作，其他旧建筑的新模型差异独立记录。下列显式模式优先于默认兼容规则：

```json
{
  "OFFICIAL_PLACEMENT": {"mode": "strict"},
  "XML_STRUCTURE": {"mode": "strict"}
}
```

模式允许 `strict` 或 `diagnostic`。未知项不记为合格。每个包的实际 `overrides.xml` 内容、`map.xml` 内联资产及非 SDK 本地 Include 参与检查和缓存绑定；尚未解析的局部资产覆盖会使官方几何精度待核验，严格模式拒绝用原版索引代替覆盖后的事实。显式生产修复在最终封装前仍严格执行这一门禁，失败时不提交新版本。`DATA:`／`ART:` 资源引用仍按基础 SDK 来源处理，不宣称递归解析外部资产编译结果。

完整保存包的增量发布在未声明模式时沿用诊断默认，即使尚未采用为冻结制作源。后续分发和成品审计只有在发布证明仍匹配实际地图字节、版本及原有策略时才沿用该默认；显式严格模式始终优先。初次采用仍先检查原有出生点、坡口和资源门禁，再检查新增最终场景，不能通过临时配置冻结来源绕过新图要求。

报告位于本版 assets：`scene-report.json`、`official-placement.json`、`xml-structure.json`、`unit-routes.json` 和 `source/final-scene.json`。`passed` 表示生效自动门禁通过；`raw_passed`、pending、覆盖范围和独立交付验收应分别阅读。缓存绑定实际原生/XML/覆盖内容、生效配置、公共策略和官方索引身份，同时检查导出文件校验值。

```sh
cargo run --locked -- audit-scene --map <id>
cargo run --locked -- check-xml <map.xml>
```

`audit-scene` 只生成检查资料，不重建或发布地图；严格模式失败会保留失败报告并返回错误。`check-xml` 只读输入并输出结构子集报告。

## 官方地图语料

普通测试读取 `tests/fixtures/official-map-fragments.zip` 中四张官方图的 SHA-256 绑定片段，覆盖不同边框、440×640 矩形、环境区块版本和未知区块保留，不要求本机存在整份镜像。片段明确区分原始字节与为最小测试构造的数据，不把小夹具称为完整官方地图。

```sh
python3 tools/verify_official_maps.py --check-fixture
# 显式验证本机完整镜像；缺失镜像会报告 unsupported 并失败
python3 tools/verify_official_maps.py --verify-full
```

官方图用于输入兼容和编解码回归，不要求它们满足本项目八席、48 矿等设计规则。矩形导航按实际宽高遍历，不改变项目新图的正方形制作规范。
