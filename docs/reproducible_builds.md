# 可复现构建与测试输入

当前工程仅保留河南当前可玩成品，其余地区保留地形预览用于新图设计。完整制作输入在maps/河南，首次检出不依赖旧assets、dist、work、历史地图包或游戏安装目录。

## 首次检出与CI

```sh
cargo run --locked --example test-data -- restore
cargo run --locked --example test-data -- check
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

恢复工具只读取当前河南配置和authoring/source.zip，检查原生CRC及所需文件，将5个游戏文件和source/layout.json恢复到当前assets目录。已有相同内容复用，已有不同内容则报错，不覆盖制作成果；不再支持旧历史素材清单、旧ZIP恢复或capture命令。恢复不修改源包、矿位、脚本或版本，不依赖网络及游戏安装。

当前源包内layout.json和工具生成的布局可能只有JSON排版差异，恢复按当前布局的标准序列化核对；5个游戏文件必须保持源包原字节。CRC用于发现误损与版本不一致，不替代来源审核。

## 受控输入

| 位置 | 用途 |
|---|---|
| maps/河南/config.json、version.json、coordinate-system.json | 当前配方、版本和可玩Frame |
| maps/河南/authoring/source.zip、layout.json、baseline.json、resource-distribution.json、coordinate-id-map.json | 完整当前原生源、对象布局、零问题基准、两矿分级计划和原生编号 |
| maps/河南/histories/v3/layers.npz及根layers.npz | 当前选定蓝图原件与原字节镜像 |
| `maps/河南/histories/<当前版本>/` | 当前成品的布局、规则、验收与预览资料 |
| maps/*/data/terrain-preview/及基础预览图 | 地区地形设计、原始高程／轮廓、分层和来源证据，不能当作可玩成品 |
| templates/ | 当前地图骨架、公共脚本与资源规则；已移除旧成品专用规则模板 |
| tests/fixtures/official-map-fragments.zip | 独立格式片段，用于编解码验证，不恢复旧可玩地图 |

assets、dist、work和target是生成物或本地工作目录，不作为首次检出的制作来源。新增新图输入须精确纳入Git白名单，不恢复旧地图目录或依赖被删除的旧打包例外。

## 当前构建与封装

```sh
cargo run --release --locked -- build --map 河南
cargo run --release --locked -- accept --map 河南
```

所有入口的生效规则与自动覆盖按[公共约束总表](map_rules.md)核对，完整原生包不豁免当前问题。配方已拒绝的历史字段和值见[Rust 配方](rust_migration.md#当前严格验收与格式保护)；旧二进制格式的读取能力服务文件编解码，不授权旧成品绕过制作规范。

先按[公共交付验收](map_acceptance.md)核对本轮差异和同版证据，再按[成品编辑衔接](coordinate_pipeline.md#成品编辑后的衔接)采用完整当前源并重建；核对 5 个游戏文件在 assets、dist、ZIP 和冻结源逐字节一致。纯 XML 分组顺序规范化保留每个元素的完整内容，须单独保存前后差异和当前校验证据，不修改原生地图玩法。

测试使用当前河南或合成场景，覆盖完整占地、坡口、两矿阶段、矩形坐标、原生局部编辑、完整封装拒绝、缓存失效和事务回滚。不会通过恢复旧地图、保留旧失败阈值或跳过检查使旧成品通过。测试实际结果以本轮日志为准；不要求启动游戏或GUI。
