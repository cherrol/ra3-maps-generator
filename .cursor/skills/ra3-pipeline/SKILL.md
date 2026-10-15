---
name: ra3-pipeline
description: >-
  地图构建、验收、版本和产物清理。跑 build_all、accept、bump、安装地图或清理
  assets/dist 时使用。地图 id 来自参数或 maps/.default，不写死某一张图。
---

# 流水线

从地形预览做到可玩地图的顺序见作图技能。构建落点是：高度层和水域，斜坡和登陆岸，出生点，近矿，近处油井，外围矿，城市油田和房子，哨站和精兵学院，车库、医院、机场，船厂，纹理，装饰（层交界、水陆交界、树和石头）。本技能只记构建命令。

`tools/build_all.py`：地形 → 连通性 → 组装 → 校验 → 预览 → 验收 → 说明 → 打包。验收以 `tools/accept.py` 为准，不过则构建失败。地图用 `--map <id>`，缺省时读 `maps/.default`。每次打包都先 `--bump`，发布名带新版本号，再装进游戏。不覆盖已经安装的上一版。

```
python -X utf8 tools/build_all.py --map <id> --bump --install
```

还没进完整 `.map` 的地形预览：

```
python -X utf8 tools/relief_preview.py --map <id>
```

步骤和缺省阈值在 `tools/relief_preview.py`。单张图用 `PREVIEW` 覆盖，水域保留框用 `COAST_KEEP`。预览只留 `source.png`（真实高程）、`preview.png`（游戏高度层）、`preview--line.png`（带边界），不写中间版本。

Python：`C:\Users\cherrol\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\python\python.exe`（无 scipy）。本机命令需要关闭沙箱。PowerShell 会吞掉内联 Python 的引号，多行逻辑写成文件再跑。

## 版本

`--bump` 先写新版本号，失败则滚回该图的 `version.json`，并删掉这次新出现的资源目录和 zip。玩家可见的地图名带版本号，不覆盖已安装的上一版。

## 清理

仓库里的历史产物：`python work/cleanup.py --prune-assets`，只留当前版本。不要删除游戏 Maps 目录里其他地图的文件夹。

## 高度来源

分层蓝图存在并被配方启用时，它就是高度来源。DEM 和谷歌地形底图只做对照，不驱动现行高度，除非该图的配方明确改用它们。
