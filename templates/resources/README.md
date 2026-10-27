# 原版资源目录

`stock-global-objects.json` 记录 RA3 1.12 `WBStatic_12.big` 中全局预加载的 GameObject 名称。`stock-object-sources.json` 中的来源路径来自 `WBData_12.big` 的资源清单；其中两个废墟名称是其源文件内定义的别名。它们是游戏资源标识，不包含本机安装路径。

资源生成规则由用户确认正常的 `[CHAI]叠岭连原T0.5.1--rebuild.zip` 校准：

- 同名对象去重，排除航点、道路及全局预加载对象。
- 补入脚本动作 ObjectType 参数（类型 15）所创建的非全局单位；否则会漏掉水上油井、船厂科技建筑、教学犀牛坦克等依赖。
- 加入环境贴图、音乐类型和实际道路。
- 用游戏 ASCII 小写 FastHash 算实例 ID，按 `(TypeId, InstanceId)` 排序。整个 25 项清单与正常包逐字节相同，测试在 `tests/repack.rs`。
- 同时从实际对象、玩家列表、脚本和纹理表导出完整 `map.xml`；纹理 ID 与逻辑纹理名的 FastHash 一致，保留原图集的 diffuse/normal 路径。

后续新增非全局对象时，在地图配方的 `OBJECT_RESOURCE_SOURCES` 中提供准确的 `DATA:...xml` 来源。已有全局对象可用 `null`，无须重复导入其定义。未知依赖不再静默跳过。
