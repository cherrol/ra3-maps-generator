# 河南 T0.0.5 交付验收

候选原生 SHA-256：`92fa436cee1a5f28ae9c9fb9e198de20533b245b246a8767ae214b369d89da70`；日期 2026-10-29。Frame 840×840、边框 20、可玩 800×800。政策、布局与计划哈希见 acceptance.json。

| 必检项 | 结果 | 同版证据 |
|---|---|---|
| 布矿及完整出生组 | 通过：48 矿、每位六阶段、家区三矿、零出生组问题 | resource-distribution.json、checked-resource-distribution.json、checked-spawn-placement.json、native-audit.json |
| 全宽坡面、端口及完整资源占地 | 通过：14 坡、零发现项 | spatial-report.json、native-audit.json |
| 整圈岸线与单层包裹 | 通过：双湖 356 段全部记录，全周双向连通；逐段有回到所属台地的原生路径 | shoreline-acceptance.json（含调整前接岸统计和分层归属） |
| 热点双方实际路程 | 通过：四对双方到八个相同目标，各保留实际路径、接入点和长度 | hotspot-path-comparison.json |
| 全部建筑朝向 | 通过：16 座预置建筑、286 条建筑动作，允许 45/135/-45/-135 度，误差 <0.001 度 | building-facing-acceptance.json、checked-production-facing.json、checked-production-spawns.json |
| 脚本保护与完整封装 | 通过：完整脚本树仅定向创建角度变化；五文件在 assets、dist、ZIP、冻结源中一致 | script-preservation.json、build-verification.json |

双方热点路程差异逐目标记录；第 2 对其中一个目标偏向西侧，另一目标偏向东侧，保留两座不同争夺点。无公共额外路程比率上限，实际值未以直线替代。

面积单列本轮明确例外：按照用户东西红圈及保留双湖的要求定稿；陆地差约 0.49%，水域差约 837.30%，不声称满足默认水域面积均衡。完整数据见 team-area-balance.json。右上内移已获用户明确选择，其余微调均在圈内。

命令自动验收与上述独立验收分别保留；本结论仅覆盖原生数据及静态通行，不包含游戏或 GUI 启动。
