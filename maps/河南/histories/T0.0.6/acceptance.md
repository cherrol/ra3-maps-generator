# 河南 T0.0.6 交付验收

日期 2026-10-30；原生 SHA-256：`d3c04978b80d7104046384a1469b10ca8eabb4b120c99e3203093fc42027fb64`。Frame 840×840、边框 20、可玩 800×800。生效参数及本版布局、分级计划身份见 acceptance.json。

| 必检项 | 结果 | 同版证据 |
|---|---|---|
| 建筑朝向（含全部矿脉） | 通过：48 座矿脉、16 座其他预置建筑及 286 条建筑动作均为四斜向，容限 1e-6 弧度；赠送动作同时朝向对方中心 | ore-facing.json、building-facing-acceptance.json、checked-production-facing.json |
| 布矿与完整出生组 | 通过：48 矿、每位六阶段、家区三矿；8 个出生点保持，全部出生组零问题 | checked-resource-distribution.json、checked-spawn-placement.json |
| 坡面、端口及完整占地 | 通过：14 条 24 格山地通道，坡形、坡口、完整矿场矩形和赠送避让零问题 | spatial/report.json、native-diff.json、gift-intrusion-exclusion.json |
| 交战路线净宽 | 通过：四组主次共 8 条实际路径逐格支持至少 10 格净宽；保守叠加全部 110 个潜在赠送点及出入口预留后仍通过 | corridor-width-acceptance.json |
| 细长伸入主体 | 通过：设计层、水陆及实际有效水陆域无未处理的 5 格以下细长部位或窄颈；15 处固定标高带诊断逐处核实为既有坡道／湖岸过渡，保留坐标、标高及来源 | terrain-shape-acceptance.json |
| 赠送建筑避开伸入区域 | 通过：110 个主备点、280 条创建分支；完整旋转地基与出入口预留落在明确主体安全区，避开矿场和全宽坡口 | gift-intrusion-exclusion.json |
| 交战区与资源接入便利 | 通过：62 个资源目标、496 条同域实际资源路径完整留证，主次交战区按 8 条全宽路径接入；地理绕行如实记录 | access-convenience.json |
| 整圈岸线及单层包裹 | 通过：双湖全部 356 段逐段双向过渡及所属平台回接；输入和岸线规则未变化，原生逐点再核 | shoreline-acceptance.json |
| 热点双方实际路程 | 通过：4 对玩家到 8 座相同热点分别留实际路径与长度；差异按 max(dA,dB) 为分母记录 | hotspot-path-comparison.json |
| 脚本保护及封装 | 通过：182 个对象身份、归属、属性保持；完整脚本树仅 14 条既有赠送动作 ANGLE 改变；完整原生运行时封装通过 | script-preservation.json、native-diff.json、build-verification.json |

本轮实际调整：47 座矿脉角度修正，13 座矿同层微移，最大 18.44 格；110 个赠送主备点中 103 个重新落位，最大距原点约 354.35 格，均在原座位 400 格公共搜索范围内。赠送主备资格、阵营、条件及占用重试机制保持。

原生仅 ObjectsList、PlayerScriptsList、TriggerAreas 改变。高度、材质、通行／建造位图、水域及坡道数据与 T0.0.5 逐字节一致；所选 v3 蓝图及根镜像保持原字节。核对过渡诊断不把窄高程轮廓条当作新增独立平台，也未删除原诊断来填通过。

面积继续列明已有用户选择：保留东西八席和双湖，陆地面积差约 0.49%，水域面积差约 837.30%；不声称满足默认水域面积均衡，例外来源和原测量另存 team-area-balance.json。

以上为原生数据与保守静态模型验收，未启动游戏或 GUI。实际动态建造、寻路与 AI 行为不由该静态记录推断。

[矿口方向核验图](ore-facing-review.png)：白箭头为原生角度翻 Y 后的矿口方向，青框为完整矿场规划范围，金点为矿脉，粉圈为出生位。

补充对照新增预览筛查：138 条短断面已逐项归属到完整二维主体或短圆整封端；腐蚀核心未分裂，所有移除边角块的直径上界均小于 5 格。原预览批检原稿保留，其可玩路线和赠送范围由本版原生记录补齐，见 preview-width-reconciliation.json。
