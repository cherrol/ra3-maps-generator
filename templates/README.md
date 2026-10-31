# 公共参考模板

当前参考来源为用户指定的 `[AxAI]4v4_巴顿码头v1.21`，于 2026-10-31 从同名原生 `.map` 导入。

| 文件 | 内容 |
|---|---|
| `template.map` | 原生参考地图的完整原字节副本：560×560、边框10、26个区块、35种纹理、5362个对象 |
| `scripts.bin` | 从同一地图提取的 `PlayerScriptsList` 区块 payload，不含区块头；20个列表、253条脚本，名称索引依赖 `template.map` 的字符串表 |
| `scripts.txt` | 同一脚本树的列表、组名和脚本名目录，保留原始顺序与名称 |

来源地图 SHA-256：`5d821900fb69c04a036488fad7b47f01fd5a408f2a97b5c379d2182a94ef7d82`。

脚本 payload SHA-256：`07e4aee47e7991e9f5bbafee692510cfc5fd284d63fc92620a82b9749d4c88cf`。

这是一张完整社区参考地图。新图构建按本图配置替换地形、对象、水域及 `RULESET` 指定的玩家、队伍和脚本；其他区块继续保留，使用时核对其适用范围。替换参考来源不自动将原图的AI、外交或赠送玩法加入现有地图，也不修改已有地图的冻结制作源。

`scripts.bin` 与 `scripts.txt` 用于保留和查阅参考脚本；当前构建的脚本来源仍是 `rules/<RULESET>.json`。脚本清单可用 `ra3maps dump-scripts templates/template.map` 回读，区块与对象统计可用 `ra3maps inspect templates/template.map` 核对。

全部253条参考脚本（含公共列表）已按原生玩家完整提取到 [scripts/](scripts/README.md)；当前224条公共规则脚本另存于 [scripts/common/](scripts/common/index.json)。两套都保留空列表、原始绑定及引用清单，由各自索引记录来源与顺序；构建仍读取原有 `RULESET`。

`resources/` 保存构建使用的资源索引，说明见[资源目录](resources/README.md)；`worldbuilder/` 保存WB导出配套文件。
