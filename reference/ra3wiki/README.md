# 本地 ra3wiki

来源是[红色警戒3 地图编辑器 wiki](https://www.yuque.com/muzeqaq/ra3mapwiki)，抓取日期 2026-10-15。

查地编脚本时先看这里，再打开原文。教程、Lua 库说明和视频不在本地，用目录里的链接打开。

| 文件 | 内容 |
| --- | --- |
| `NAV.md` | 全书目录。已抓的条目直接链到本地文件，未抓的链回语雀 |
| `catalog.md` | 全书目录。已下载的条目链到本地文件，未下载的链回原文 |
| `catalog.json` | 同上，带分组路径；已下载的条目多一个 `file` 字段（表格页另有 `sheet`） |
| `pages.json` | 抓取索引：标题、slug、目录路径、原文链接、本地文件、字数、原文更新时间 |
| `pages/` | 详情页正文，一页一个 `.md`（187 个），表格页另带 `.sheet.json` |
| `actions.json` | 脚本动作。563 条 |
| `conditions.json` | 脚本条件。155 条 |

`catalog.json` 的 `href` 始终是语雀原文地址，抓取脚本靠它取 URL；本地地址在 `file`
（表格页的原始表格数据在 `sheet`）。带说明的行只有三种：`表格页`、`原文正文为空`、`未下载`。

## 详情页（`pages/`）

文件名就是页面原标题，例如 `pages/日冕升级列表.md`。每个文件头部有 YAML：

```
title / catalog_path / slug / source / fetched / content_updated_at / type / format
```

标题里 Windows 不允许的字符换成同形全角字符（只有 `/` 会碰到，例如
`pages/玩家／队伍 数据导入导出.md`）。文件名的映射规则在 `names.py`，`slug` 始终留在
文件头里，所以按语雀地址或编号反查也不难。

正文是语雀的原始 markdown（`mode=markdown` 导出），表格、代码块、公式都保留。
正文里的图片只留图片地址，没下载图片本身。

已抓 187 个页面，跳过 49 个（`地编教程` 下的技巧、图文教程、视频教程共 51 条里的 49 条，
另加第三方地编的 `MapEditor(第三方地编)`；列表见 `pages.json` 的 `skipped`）。

三类特殊情况：

- **表格页**：语雀的 `lakesheet` 页没有 markdown 正文，正文在 `content` 里，是 zlib 压缩的
  表格数据。这些页已解压成 markdown 表格（跨行跨列合并按平铺展开），原始数据另存同名
  `.sheet.json`。共 5 个：`Weapon清单（适用于地图使用）`、`组合式AttributeModifier清单`、
  `日冕单位名称`、`日冕新增PackedTextureImage 清单`、`2.99D_冲突Mod地编相关(含增援代码)`。
- **目录/索引页**：11 个页面在语雀上正文本来就是空的（`数据揭示`、`helper`、`base_module`、
  `object`、`玩具们`、`日冕数据揭示` 等），它们是纯目录节点，子页已单独抓取。文件里有说明。
- **`TextureImage图鉴801-1000`**：原文正文为空，可能是作者没写完，其余 4 段图鉴（1–800）都有内容。

## 动作和条件

动作和条件每条有这些字段：

- `index`：动作或条件编号
- `id`：命令字，例如 `CREATE_UNNAMED_ON_TEAM_AT_WAYPOINT_WITH_ORIENTATION`
- `path`：编辑器里的菜单路径
- `zh`：中文名
- `arg`：参数提示，`${...}` 是参数类型
- `obsolete`：编辑器把它标在 Unused or Obsolete 下面时为 true

参数以 `actions.json` / `conditions.json` 的 `arg` 为准。中文名来自[动作列表](https://www.yuque.com/muzeqaq/ra3mapwiki/orutriyre2fllrih)和[条件列表](https://www.yuque.com/muzeqaq/ra3mapwiki/fnshx90tfo5m4k8k)。有 37 条动作、6 条条件只有中文名和编号，参数提示为空。那几条的参数块在原文里换了行，没有收进表。

## 重新抓取

抓取脚本在 `work/yuque_fetch.py`（工作脚本，不在 `tools/` 里）。
它读 `catalog.json`，用公共文档接口 `https://www.yuque.com/api/docs/<slug>?book_id=<id>`，
文档页再取 `mode=markdown` 的 `sourcecode`，表格页解压 `content`。
改完跑一次即可增量更新，然后依次跑

```
python docs/ra3wiki/link_catalog.py    # 目录改指本地文件
python docs/ra3wiki/rename_to_title.py # 文件名按标题对齐（幂等）
python docs/ra3wiki/make_nav.py        # 重建 NAV.md
```

`rename_to_title.py` 支持 `--dry-run` 只打印改名映射。它发现 `pages/` 里有索引之外的
文件时会直接停下报错，不会乱改。
