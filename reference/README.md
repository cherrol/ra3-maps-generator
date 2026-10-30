# 外部参考资料

本目录只保存供查阅的第三方资料，**不参与构建、验收或运行逻辑**：源码不读取这里的文件，`cargo` 不编译这里的代码，测试不依赖这里的字节。

| 目录 | 内容 | 来源 |
|---|---|---|
| `opensage/` | OpenSAGE 的 C# 地图数据解析器（`MapFile`、`BlendTileData`、`GlobalVersion`、脚本结构等） | 上游 OpenSAGE 项目，用于核对原生 SAGE 格式字段 |
| `ra3wiki/` | 离线 RA3Wiki：编辑器脚本动作、条件、单位与材质索引 | <https://www.yuque.com/muzeqaq/ra3mapwiki>，抓取日期见其 README |

使用边界：

- 实际地图格式以 [地图格式规范](../docs/spec/ra3_map_format_spec.md) 和**真实字节**为准；两者冲突时，以本项目实际读写结果为准。
- EA 官方数据镜像不在本目录，见本地 `data/ea-modding/`（不入库，`tools/fetch_ea_modding.sh` 重建），说明见 [官方数据镜像](../docs/ea-modding/README.md)。
- 本目录任何文件的更新都不得改变构建产物；若某条规则需要复用这里的内容，应先把结论固化进 [公共约束总表](../docs/spec/map_rules.md) 与 `rules/`，而不是让代码直接读取这里。
