# 完整脚本提取

分别保存[当前参考模板](../README.md)与当前公共规则的完整脚本。按各自原生 `SidesList.playerName` 确定列表位置，不从出生位编号推断，也不混合两套来源。

| 来源 | 提取位置 | 列表数 | 脚本数 |
|---|---|---:|---:|
| 巴顿码头参考地图 `templates/template.map` | 当前目录，[完整索引](index.json) | 20（含12个空列表） | 253 |
| 当前公共规则 `rules/common-T0.5.1.json` | [common/](common/index.json) | 19（含14个空列表） | 224 |

`index.json` 按原生顺序记录每个列表的玩家、文件、脚本数和SHA-256，并保存原字符串表及完整脚本区块的身份。空列表也单独保存，保证重组时位置不变。`neutral.json` 对应公共列表 `ScriptList[0]`；源玩家名为空时，工具使用 `_neutral_` 显示该身份。

## 参考地图脚本

| 文件 | 玩家／列表 | 脚本数 | 内容 |
|---|---|---:|---|
| [neutral.json](neutral.json) | 公共列表0 | 52 | 初始化8条、固定苏联AI21条、固定帝国AI建设23条 |
| [plyr-civilian.json](plyr-civilian.json) | `PlyrCivilian` | 1 | 投毒事件 |
| [plyr-creeps.json](plyr-creeps.json) | `PlyrCreeps` | 94 | 帝国设施查找、科技生产、恢复、防御及海陆处理 |
| [player-1.json](player-1.json) | `Player_1` | 1 | 牛蛙支援 |
| [player-2.json](player-2.json) | `Player_2` | 1 | 牛蛙支援 |
| [player-3.json](player-3.json) | `Player_3` | 1 | 牛蛙支援 |
| [player-5.json](player-5.json) | `Player_5` | 51 | 阵营增强，原列表17 |
| [player-6.json](player-6.json) | `Player_6` | 52 | 阵营增强，原列表18 |

其余12个空列表见索引。此前已提取的P5／P6文件继续保存在原路径。

## 当前公共规则脚本

| 文件 | 玩家／列表 | 脚本数 |
|---|---|---:|
| [common/neutral.json](common/neutral.json) | 公共列表0 | 173 |
| [common/player-5.json](common/player-5.json) | `Player_5`，原列表15 | 4 |
| [common/player-6.json](common/player-6.json) | `Player_6`，原列表16 | 4 |
| [common/plyr-civilian.json](common/plyr-civilian.json) | `PlyrCivilian`，原列表17 | 17 |
| [common/plyr-creeps.json](common/plyr-creeps.json) | `PlyrCreeps`，原列表18 | 26 |

其余14个空列表见公共索引。公共规则导出时从该JSON自己的玩家表、字符串表和脚本树生成原生列表，不借用参考地图的玩家顺序或玩法。索引绑定规则JSON的SHA-256；源中记录的上游地图身份仍以原规则文件为准。

公共构建工具按地图布局动态生成的 `gift-clear-*` 辅助脚本，以及当前未启用的 `minute-attack-*` 模块，继续由其Rust生成器维护，不虚构成固定脚本快照；这次导出覆盖当前规则中实际声明的全部224条脚本。

## P5／P6阵营增强组成

| 提取文件 | 原生玩家 | 原始列表索引 | 脚本数 | 组成 |
|---|---|---:|---:|---|
| [player-5.json](player-5.json) | `Player_5` | 17 | 51 | 查找及阵营启用4条、帝国15条、盟军21条、苏联11条 |
| [player-6.json](player-6.json) | `Player_6` | 18 | 52 | 查找及阵营启用4条、帝国14条、盟军22条、苏联12条 |

JSON 中的 `scripts` 直接使用现有 `scripts::Node` 模型，包含一个完整 `ScriptList`。保留全部子组、名称、版本、顺序、条件、真假分支、动作、参数和原始启用／停用标记。`source` 记录源地图及原始列表的 SHA-256；`references.by_argument_kind` 汇总实际字符串参数，包含外部依赖、内部定义与读写引用，尚未解析成完整依赖图。

原生中文字符串继续使用项目编解码的一字节 Latin-1 映射；`prefix`／`body` 中的原始字节不做转码。人工查看中文名称时按原图GBK解码，完整目录见[参考脚本目录](../scripts.txt)。

## 接入前需要绑定的上下文

- 玩家与队伍：两份脚本分别使用 `teamPlayer_5`、`teamPlayer_6`，并存在对另一玩家的检查；迁移时应检查全部引用，不仅替换文件所属玩家。
- 时间：两份都引用公共 `/Game Start`；另分别使用 `T3Time`、`T3Time2`。公共时间系统未随本次提取复制。
- 航点与区域：例如P5的 `Warfactroy1`、`airf1`、`airf2`、`kk5`、`Area 开矿`，P6的 `Warfactroy2`、`6KK`、`6airf4`、`Area 开矿2`；完整清单在JSON中。
- 具名对象及动态引用：基地、工厂、机场、扩矿单位等名称继续使用原图约定；查找动作创建的引用与地图预置对象要分别处理。
- 脚本／组名、命令按钮、升级及对象类型：保持原始拼写与作用上下文；重命名或跨图装配前需重新链接和核对。

所有提取文件都是原样数据快照，当前尚未参数化；构建继续读取原有 `RULESET`，不自动采用这些提取文件。启用条件继续按原脚本执行，提取工具不新增“仅AI”条件或改写归属。

## 重新提取与核对

```sh
# 全部参考脚本，含公共列表和空列表
cargo run --locked --example extract-player-scripts -- templates/template.map --all templates/scripts
cargo run --locked --example extract-player-scripts -- templates/template.map --all templates/scripts --check

# 全部当前公共规则脚本
cargo run --locked --example extract-player-scripts -- --rules rules/common-T0.5.1.json templates/scripts/common
cargo run --locked --example extract-player-scripts -- --rules rules/common-T0.5.1.json templates/scripts/common --check

# 仍支持单独提取指定玩家
cargo run --locked --example extract-player-scripts -- templates/template.map Player_5 templates/scripts/player-5.json
cargo run --locked --example extract-player-scripts -- templates/template.map Player_6 templates/scripts/player-6.json

cargo run --locked --example extract-player-scripts -- templates/template.map Player_5 templates/scripts/player-5.json --check
cargo run --locked --example extract-player-scripts -- templates/template.map Player_6 templates/scripts/player-6.json --check
```

工具每次都验证原始列表逐字节往返一致，并使用全新的字符串表验证重链接后脚本树一致。整套导出额外将全部列表依原顺序重组，与完整源脚本区块逐字节比较；`--check` 再核对已保存的全部JSON与索引。整个过程不修改源地图、原规则或游戏目录。
