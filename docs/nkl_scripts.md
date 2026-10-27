> 此文保留迁移前的设计记录；其中 Python 工具路径已停用。当前命令与数据格式见 [Rust 迁移说明](rust_migration.md)。

# 三系脚本分类（NKL / SMAI / AXAI）

本机地图库里三个系列共 83 张图：NKL 16、SMAI 22、AXAI 45。作用根据脚本名和它所在的组来判断，没有逐条解开脚本动作的参数。同一张图的多个版本骨架相同，下面按图概括，不按版本重复。

脚本列表的序号就是该图玩家列表里的序号。0 号几乎总是公共脚本。11–16 在多数 4v4 里是 `Player_1` 到 `Player_6`，巴顿码头、死亡搁浅、holiday ocean 的玩家顺序不同，同一套建造会挂在别的序号上。固定额外位是 `PlyrCivilian` 和 `PlyrCreeps`。

## 共用的十类

三系补的是同一件事：认阵营、给钱、把厂和兵补出来、再推出去打。下表是以后对照时的目录。某一格为空，表示这个系列基本不靠这类脚本。

| 类 | NKL | SMAI | AXAI |
|---|---|---|---|
| 开场、音乐、雷达 | 署名、`BGM`、`radar`、台词 | `Map By AUX`、`BGM` / `Music`、`Radar` | `FE`、`obj`、少量 `Music` |
| 钱 | 人类只显示；电脑分拍发。苏联五拍、帝国六拍 | `AI Money`，另有 `Money S0`–`S6`、`Money E0`–`E7` | `money`、`money11`，以及按矿场数量展开 |
| 阵营和基地车 | 删掉错误基地车，或再发一辆。`IF AI` 才发育 | `SovietMCV List` / `JapanMCV List` / `AlliedMCV List`，`IF P Allied`，`UNPACK` | `帝国启用` / `盟军启用` / `苏联启用`，`Japan MCV On` / `Soviet MCV On` |
| 发育 | 按座位写死厂和特征单位，缺了再补 | `Find` 之后 `Build`：兵营、重工、船厂、机场、矿场、科技 | 每个厂一条生命周期：建造、到点激活、关闭、摧毁或丢失后再造 |
| 进攻 | `2WAR` / `3WAR`，先找到部队再下令 | `Hunt` | `苏联_War`、`造兵`，终末阵地另有 `AI Hunting Script` |
| 计时 | `Timer` 到点打开科技、波次、禁令 | `TIMER GAME START` 及 ADD | `时间`、`Time_Game Start`，每个厂自己的 `T2TIME` / `T3TIME` |
| 外交 | `AI Relationship`，按随机阵营把 P5/P6 算进同伴 | `Relation` | 基本不改敌友，用房间里的队伍 |
| 人类侧 | 油井、特殊能力、死亡补偿 | 少。增益写成 `Allied Buff5` 这种座位号 | 杀敌赏金、哨站、电站奖励挂在人类座位上 |
| 单图事件 | 残骸、日冕、磁暴、NS 事件 | 任务塔、摄像机、中立单位、战役残留 | BOSS、英雄、协议、蜂巢、蜘蛛、全图视野 |
| 未命名或看不出 | 短图里的 `Script N` | 少量 `Script`、`R.D` | `FE` 只知是启动入口 |

挂法不同，取用时先看挂法再看类：

- NKL、SMAI 把整棵树放在公共列表，用座位名和阵营名分支。
- AXAI 把发育状态机挂在「该强的那个座位」自己的列表上，公共列表留一份阵营模板和这张图的玩法。人类座位通常只有奖励。

## NKL

16 张图、13 棵不同的脚本树。anise ev2.00 与 ev2.01、empire cemetery 1.04 与 2.00、industrial fantasy remake 与 remake 1.2 各是同一棵树。笔误按作者原文保留：`Hunam` 即 Human，`emeny` 即 enemy。

成熟图放在公共列表，用座位名分支。短图挂在 `PlyrCivilian`、`PlyrCreeps` 或某个 `Player_N` 上。

### 座位约定

成熟图把八个座位分成两侧：

| 座位 | 在脚本里的角色 |
|---|---|
| P1–P4 | `Look Money / Human`。只有该格是电脑时，`IF AI` 才给建造和进攻 |
| P5–P6 | `Look Money / AI`。按盟军 A、苏联 S、帝国 J 分叉 |
| P7、P8 | 额外位，各自一条线。常见搭配是 P7 帝国、P8 苏联；dream exile、eyes 里 P7 走盟军；empire cemetery 里两个都走苏联；twisted magma 的 P7、P8 再各分盟军和帝国 |

### 1. 开场、音乐、雷达

开局提示和氛围，不参与生产和进攻。

| 脚本 | 作用 |
|---|---|
| `Map Made by NKL…`、`hello olg`、`hello1`、`GLHF`、`Sc2ra3` | 开局字幕或说明 |
| `BGM`、`BGM SET`、`BGM 2`、`bgmm`、`BGM RE`、`bgm stop`、`RE` | 放音乐、换曲、停止。单独的 `RE` 在 BGM 组里，是重播或收束 |
| `radar` | 开雷达 |
| `talk 1`、`AI talk 1MO`、`AI talk 1OL`、`AI talk 1 JA` | 台词。MO / OL / JA 按阵营各一句 |

出现在几乎每张有署名的图上。

### 2. 钱

人类侧和电脑侧分开写。

**人类只显示。** `Look Money / Human` 下的 `P1 M`、`P1 M Show` 到 `P4 M`。给这四个座位弹出金钱文字。

**电脑侧发钱，也带一条显示。** `Look Money / AI 1` 下的 `P5 M` 到 `P8 M`，以及对应的 `M show`。组名把这四个座位划进 AI。

**按阵营分拍发放。** `Money` 组里的 `Soviet1`–`Soviet5`、`Japan1`–`Japan6`。编号是同一套津贴的第几拍，苏联五拍、帝国六拍。anise 2.05、blizzard、frontal 2.05 都有。

**额外位单独一笔账。**

| 脚本 | 作用 |
|---|---|
| `START MONEY J` | P7 帝国线的启动资金 |
| `RR4 MONEY`、`p5 more money`、`p5 s more money` | 中后期或某个随机方案再补一笔 |
| `Money7` 的 `Script 4`–`Script 4__4`，`Money8` 的 `Script 6`–`Script 6__5` | P7、P8 各一组分拍发钱。dream exile、eyes |
| `decline 7 test`、`decline 8 test` | 同上两张图里的衰减试验，和正式发钱分组并列 |
| `money1`、`money2p7`、`money2p8` | empire cemetery 给 P7、P8 的两笔 |
| `p7 money t333`、`p8 money t333`、`p8 s money33333`、`p8 s money4444`、`Money777`、`Money888` | twisted magma 按 T3 时段给 P7、P8 加钱 |
| `mapAImoney` | holiday ocean 里唯一能从名字看出是发钱的脚本 |

### 3. 阵营识别和基地车

开局往往预放了不止一个阵营的基地车。脚本先认出该座位的阵营，再删掉不对的，或直接再发一辆对的。

| 脚本 | 作用 |
|---|---|
| `FIND MCV`、`IF 5 A`、`IF 5 S`、`IF 6 A`、`IF 6 S` | 查 P5、P6 是盟军还是苏联 |
| `remove 5 s mcv`、`remove 5 a mcv`、`remove 6 s mcv`、`remove 6 a mcv` | 删掉与阵营不符的基地车 |
| `give JMCV`、`give SMCV`、`P5 J MCV`、`find P5 J MCV`、`Give MCV` | 发给对应阵营一辆基地车 |
| `Move MCV`、`MCV MOVE TO`、`Move MCV P8S`、`mcv find`、`mcv move to`、`P8mcv find`、`p7MCV find` | 把基地车挪到落点，或确认它已经展开 |
| `Random`、`Random 1`–`Random 4`、`Random 3 5J`、`Random 4 5other` | 开局随机方案。`5J` 表示这次 P5 走帝国，`5other` 表示 P5 走帝国以外 |
| `R1`、`R2`、`R3`、`R34`、`R12` 前缀 | 同一件事的第几套随机方案，例如 `R34 P5 S YARD`、`R1 P8 2WAR` |
| `P1 IF AI`–`P4 IF AI` | 这格是电脑才继续后面的建造和进攻 |

anise 2.05 的 `Random` 组、frontal 2.05 的 `agreement`（`J MAP AI`、`Allied5`、`Allied6`、`J 5`、`J 6`）负责「这一局 P5/P6 算哪一阵营、和谁站一起」。

### 4. 发育：把厂补出来

顺序在各图里反复出现：基地车 → 生产建筑 → 特征单位 → 进攻。`FIND`、`Find … war1` 先确认建筑已经在，再进入下一步，避免空手下生产令。

**帝国（组名 `J`）。**

| 脚本 | 作用 |
|---|---|
| `B2`、`P5 B2`、`P1 B2` | 基地车之后、`2WAR` 之前的那一步生产建筑 |
| `yard`、`P7 YARD`、`P7 2 YARD`、`P7 3 YARD`、`japan yard t2 find`、`japan 3yard` | 船厂或主厂，按 1、2、3 座往上加。和 `naval`、`t2 ship` 写在一起时就是船厂 |
| `naomi`、`P56 naomi tinggu`、`P5 naomi tinggu sw` | 天狗这一路。`sw` 是发动或切换，不是另一个单位 |
| `Tinggu sw`、`ban tinggu`、`Lock yard tunami` | 控制天狗、海啸船厂的开关。先禁，到点再开 |
| `Japan T3`、`P7 T3`、`japan naval t3` | 帝国 T3，海军图上专指 T3 船厂 |

**苏联（组名 `S`）。**

| 脚本 | 作用 |
|---|---|
| `R34 P5 S YARD`、`P8YARD`、`P8 T2`、`P8 T3`、`7t3` | P5/P6/P8 的主厂，以及 T2、T3 解锁后的厂 |
| `P5 S 2AIR`、`P8 2air`、`p8 airport22` | 第二座机场 |
| `P5 tesla`、`P8 Tesla`、`tesla sw`、`tesla tank creat` | 磁能坦克。`sw` 为出动 |
| `P5 Crane`、`P6 Crane`、`crane`、`QZJ` | 起重机 |
| `ban niuwa`、`ture niuwa`、`ban niu gound` | 牛蛙。先禁，`ture` 是到点放开 |

**盟军（组名 `A`）。**

| 脚本 | 作用 |
|---|---|
| `p5 a yard ban`、`sw yard5`、`sw yard6` | 船厂先关着，`sw yard` 到点打开 |
| `p5 a reserve1`、`add2anti eng`、`anti enginer`、`t2 5`、`t3 5` | 防工程师的预备队，按 T2、T3 各放一拨 |
| `jarls shuiyi5`、`lisater shuiyi5`、`if jaerls5`、`if lisater5` | 水翼单位。`jarls`、`lisater` 是作者给两种水翼起的名字 |
| `airship5`、`anti air5`、`P5 R2 2AIR` | 飞艇、防空、第二机场 |
| `outpost5`、`P7 A outpost1 t2` | 前哨 |
| `FIND BAR`、`find p7 a bar`、`P7 bq1`、`1-2pk` | 兵营及其生产窗口。`bq` 和 `BAR` 写在同一组 |

`ban` 与 `unlock` 成对：没到该科技或该时间，就不许造。`BAN MINER`、`BAN RE`、`unsell`、`sell BT low defend`、`sell right` 管出售：低级防御可以卖掉，矿场和另一些建筑禁止卖。

### 5. 进攻

| 脚本 | 作用 |
|---|---|
| `2WAR`、`P5 2WAR`、`P1 2WAR`、`p5 S2 WAR`、`P5 A 2W` | 第二波出击。每个座位、每个阵营各一条 |
| `3WAR`、`P2 3WAR`、`P5 War3`、`3 war` | 第三波，通常只写在已经有二波的座位上 |
| `War2`、`War4`、`P6RE4_2war` | 同一座位的后续波次。`4` 是第四轮 |
| `Find p5 war1`、`Find p7 war1` | 先找到已经造出的部队，再下令 |
| `P5 Tinggu sw`、`P56 S tsl war sw` | 这一发用天狗或磁能部队打出去 |
| `high house attck`、`high house attck stop` | 打高台上的建筑，打完停止 |
| `46 defend`、`R1 78 defend`、`defend start`、`base defend pow1`、`P7 2 YARD__defend` | 守家或防守反击，和主动的 `2WAR` 分开 |

`2WAR` 依赖第 4 类已经把厂补出来。只有钱、没有这两条的图，电脑不会按作者的节拍出门。

### 6. 计时

`Timer` 组是总调度，本身不造兵，到点去打开上面的建造、进攻、禁令。

| 脚本 | 作用 |
|---|---|
| `P7 T3`、`P8 T2`、`P8 T3`、`unlock t3 naval`、`P56 A T3naval` | 到点打开对应科技 |
| `P7 W3`、`P7 W4`、`P78 MO` | P7 的第 3、第 4 个时间窗，以及 P7/P8 的钱窗 |
| `mission set`、`missioncore`、`mission timer`、`mission1 active` | 把地图目标拨到下一步 |
| `7p by pk1 start/stop`、`7p war1 shz start/stop`、`7p war2 dgn`、`7p war1 hy`、`7p war2 ydn` | 成对的窗口。start 允许这一类单位，stop 关掉。pk、bq、shz、dgn、hy、ydn 是不同兵种或不同批次 |
| `agreement time`、`J AI honor` | 结盟约定生效的时刻 |
| `hello 1`、`Time Start wait1`、`Time_Game Start` | 开局等待，给玩家看说明 |

带 `real` 后缀的是正式版，不带的是计时器里的预告。dream exile 把两套都留着。

### 7. 外交

| 脚本 | 作用 |
|---|---|
| `AI Relationship`、`AI relationship` | 把电脑座位之间的敌友设好。组名 `AI emeny` 表示这组负责敌对 |
| `Relationship`、`Relation` | 同上，短名 |
| `Relationship_P123456` | 只处理 P1–P6，额外位另算 |
| `Allied5`、`Allied6`、`J 5`、`J 6`、`J MAP AI` | 按这一局随机到的阵营，把 P5、P6 设成盟军同伴或帝国同伴 |

empire cemetery、eyes、dream exile、anise、blizzard、frontal 2.05、twisted magma 的 `Team` 都属于这一类。

### 8. 人类侧补偿

这些脚本在 Human、human buff、Hunam BUFF 组里，对象是玩家座位，不是电脑发育线。

| 脚本 | 图 | 作用 |
|---|---|---|
| `p4 highout1`、`P4 none`、`sea pow`、`if 4 s`、`if 4 j`、`banp4 j be` | blizzard、frontal 2.05 | 按 P4 的阵营给或收回一项能力，帝国位会禁掉某种建筑 |
| `p1 oil`、`p1 oil 11`、`Oil point` | dream exile、eyes | P1 附近油井的占领或奖励，`__1` 到 `__6` 是重复检查 |
| `special pow1`、`special powAI1`、`special pow human last hope`、`tsl tank human` | dream exile、eyes | 人类和电脑各一套特殊能力，最后一档叫 last hope |
| `core b12`、`mission core`、`core1 complete` | dream exile、eyes | 地图核心目标的完成判定 |
| `p2 high front mission`、`P2 BUFF`、`P4 BUFF`、`P5 BUFF`、`Human small buff` | empire cemetery | 给指定人类座位的小增益 |
| `p6 die buff`、`p4 buff 01`、`p4 enginer add` | twisted magma | 人死了或到点之后，给 P4、P6 补工程师或一项增益 |
| `Allies BUFF`、`P7 A BUFF AAA`、`NO buff` | dream exile、eyes | P7 盟军线的增益开关。`NO buff` 是关闭 |

### 9. 单张图自己的事件

不进通用 AI 骨架，换一张图就没有。

| 图 | 脚本 | 作用 |
|---|---|---|
| blizzard | `sea wreckage` 的 `w1`–`w6` | 海面残骸，六个点 |
| blizzard | `add random` 的 `Random a 1 s` 到 `Random a 5 n` | 五个随机事件，s / a / j / r / n 是不同结果 |
| blizzard | `BT SW 01`–`BT SW 05-6` | 一组测试用的开关 |
| blizzard | `anti steal`、`steal sw p1`、`use for steal` | 针对偷钱或工程师的反击，按 P1、P4 分开 |
| empire cemetery | `Corona`、`cor t3 p7`、`COR t4 P7` | 日冕，P7 在 T3、T4 各触发一次 |
| empire cemetery | `cibao spawn p1`、`cibao war fengyin`、`P4 cibaoW` | 磁暴相关的生成、封印、给 P4 的一发 |
| empire cemetery | `spnkkkc`、`protect spnkkkc`、`ban spnk`、`give P6` | 一个要保护的对象：先禁、再找到、再保护，并可以交给 P6 |
| empire cemetery | `Do not TR!!!`、`desable antiTR`、`antiTR` | 关掉某种反击或传输 |
| twisted magma | `NS 01 USE`–`NS 05 USE`、`NS CI 01`–`NS CI 05`、`NS D04` | 五个编号事件。USE 是使用，CI 是触发，D04 是第四个的变体 |
| twisted magma | `shua shuiyi`、`shuiyi 01 turn on` | 刷水翼并打开 |
| twisted magma | `p8 sea protect`、`p8 super pow` | P8 的海防和一次超级武器 |
| industrial fantasy | `btyun1`–`btyun3`、`BT shot`、`air btyun`、`lst btyun`、`jier airport`、`ydn cd` | 这张图的 BT 增益或兵团，以及一座机场。`ydn cd` 是冷却 |
| industrial fantasy remake | `water T2`、`water T3`、`water T4 mod`、`water Outpost`、`T34chuan`、`LT3__water` | 水上科技线：T2 到 T4、水上前哨、船 |
| holiday ocean、frontal 2.00、anise 2.00 | 大量 `Script 1`、`Script 6__2` | 编辑器默认名，从名字看不出作用。能认出的只有组名：`JPG`、`52Wa`、`2wa`、`ST`、`BT yun`、`Allies AI BTyun`、`JapanAI BT yun`、`CreepsAI` |

### 10. 名字不足以判断的条目

留在树里，但不归进上面的功能类：

`akm111`、`sui`、`xpow`、`IF wrenren`、`IF IF`、`NO MILAJI` 以外的单个 `IF`、`Script 9`、`bbibb`、`Scedfa`、`provoke`、`Team`（twisted magma 顶层那一条）、`niuwa11`。

`ban Milaji tank` 是禁一种坦克，具体型号只写了这个绰号。`ban laji`、`unlock laji` 是 P8 苏联线上先禁后放的一种单位，名字的字面是「垃圾」。`BUFF FORG5` 是给该座位的一项增益，单位没写开。

### 成熟图怎么拼

anise 2.05、frontal 2.05、blizzard 是同一骨架，blizzard 把每一格又拆细了（海军禁令、磁能、水翼、偷钱反击、海面残骸）。

```
开场 / BGM
Look Money          人类 P1–P4 只显示，电脑 P5–P8 发钱
Random 或 agreement 认出 P5/P6 阵营，删掉错误基地车
Money               Soviet1–5、Japan1–6 分拍加钱
AI
  P1–P4    仅当 IF AI：帝国基地车 → B2 → 2WAR，个别座位再 3WAR
  P5–P6    按 A / S / J：船厂或主厂、特征单位、2WAR
  P7       一条专线（帝国或盟军）：厂、T3、天狗或兵营、防守
  P8       一条专线（苏联为主）：厂、T2/T3、机场或磁能、2WAR
Timer               决定上面这些何时打开
AI Relationship     敌友
```

dream exile 1.08 和 eyes 1.10 换成「P7 盟军 + P8 苏联」，并加上 P1 油井、特殊能力和核心目标。1.08 比 1.07 多了 P5/P6 的苏联进攻波、磁能出动、盟军 T3，以及 P7 船厂。

twisted magma 把 P7、P8 再分成盟军和帝国两套，人类死亡补偿和 NS 事件另挂。empire cemetery 的电脑线只有 P7、P8 两个苏联位，P1 和 P6 的帝国脚本更短，其余是磁暴、日冕和人类增益。

industrial fantasy、holiday ocean、frontal 2.00、anise 2.00 还没有这套座位矩阵，脚本短，而且有一大半仍叫 `Script N`。

### 分图索引

| 图 | 条数 | 用了哪些类 |
|---|---|---|
| anise ev2.00 / ev2.01 | 35 | 未命名壳，挂在 Civilian、Player_5、Player_6、Creeps |
| anise ev2.05 | 145 | 钱、阵营、四座位发育、2WAR/3WAR、计时、外交、BGM |
| blizzard v1.00 | 253 | 同上，加海军 T3、磁能、牛蛙、水翼、防偷、残骸、随机事件、人类 P4 补偿 |
| frontal conflict v2.00 | 28 | 未命名壳，挂在 Civilian 与 Creeps，组名涉及盟军/帝国 BT |
| frontal conflict v2.05 | 121 | 与 anise 2.05 同骨架，另有 P4 人类补偿、P8 基地车挪位、P7 船厂与海防 |
| dream exile 1.07 | 164 | 钱、P5/P6 帝国与盟军、P7 盟军、P8 苏联、油井、特殊能力、核心目标、外交 |
| dream exile 1.08ex+ | 199 | 1.07 加上 P5/P6 的二到四波、磁能、盟军 T3、P7 船厂 |
| eyes of battlebase 1.10 | 145 | 与 dream exile 1.07 同族，P7 带盟军船厂，没有 P7 的分兵种计时窗 |
| twisted magma 2.00 | 168 | P5/P6 三阵营、P7/P8 各分盟和帝、分拍给钱、海防、NS 事件、P4/P6 人类补偿 |
| empire cemetery 1.04 / 2.00 | 71 | P1/P6 帝国短线、P7/P8 苏联、钱、磁能、起重机、日冕、磁暴、人类小增益、外交 |
| industrial fantasy v2.00 | 39 | Creeps 上的盟军/苏联、BT、P5/P6 帝国；一半仍是 `Script N` |
| industrial fantasy remake / 1.2 | 51 | 同上，加水上 T2–T4、水上前哨、禁卖 |
| holiday ocean v2.00 | 40 | 几乎全是 `Script N`，能确认的只有 `mapAImoney` |

## SMAI

22 张图，署名 `Map By AUX` 或 `Map By AUX.GLHF`。脚本在公共列表的 `AI` 组里，再按盟军、苏联、帝国拆开。mykonos 另外往 `PlyrCivilian`、`PlyrCreeps` 各挂了一份，这两张解析不完整，下面不把它们的残树当通用包。

和 NKL 的差别：SMAI 先 `Find` 再 `Build`，建筑用游戏物体名（`JapanWarFactory`、`AlliedAirfield`），不靠 `B2`、`YARD` 这种绰号。进攻收成一条 `Hunt`，不按座位写 `2WAR`、`3WAR`。

### 1. 开场、音乐、雷达

| 脚本 | 作用 |
|---|---|
| `Map By AUX`、`Map By AUX.GLHF`、`Copyright notice` | 署名 |
| `BGM`、`BGM1`–`BGM3`、`Music IF`、`Music Then` | 放音乐、按条件换曲 |
| `Radar`、`Radar01`–`Radar03`、`Radar AI`、`Radar P` | 开雷达。带座位号的只给那个电脑 |

约一半的图把雷达和开局计时放在 `Set` 组，和 `AI` 组并列。

### 2. 钱

`AI Money` 在 18 张有 `AI` 组的图里是主入口。细项按阵营和座位编号：

| 脚本 | 作用 |
|---|---|
| `Money S0`–`Money S6` | 苏联分拍。0 是起步，后面是后续几拍 |
| `Money E0`–`Money E7` | 帝国分拍，拍数比苏联多一档 |
| `Japan Money`、`Japan 05 - A` 到 `Japan 06 - D` | 帝国另一套分拍。05、06 是两段，A–D 是段内顺序 |
| `AI MoneyP5`、`AI MoneyP6`、`AI Money AlliedP5`、`AI Money AlliedP6`、`P Money` | 只给这个座位，或只给这个座位的盟军 |
| `Allies 01 Money`、`Allies 02 Money` | 同一张图上两路盟军各一笔 |

人类座位没有对应的「只显示」组。钱写在 `AI` 组里。

### 3. 阵营和基地车

| 脚本 | 作用 |
|---|---|
| `IF P Allied`、`Find Soviet`、`Find Japan` | 先确认这格是哪个阵营 |
| `AlliedMCV List`、`SovietMCV List`、`JapanMCV List` | 该阵营基地车的名单，用来查找或发放 |
| `UNPACK` | 基地车展开 |
| `Build SovietSurveyor`、`Find SovietSurveyor` | 苏联勘探车。先找，没有再造 |
| `AI ON` | 整包电脑脚本的总开关 |
| `SovietLock`、`AlliesLock`、`Allies AI Shiplock` | 锁住不该走的阵营或海军 |
| `Tech2 Unlock`、`AlliedTechUnlock`、`SovietTechUnlock`、`JapanTechUnlock` | 到点才允许该阵营的下一级 |
| `AI Skill Lock`、`Tech Lock`、`Build Lock P`、`P Unit Lock` | 把难度或可造单位锁住，不跟着房间难度掉下去。`AI Skill Lock` 写在 massacre valley |

### 4. 发育

模式固定：`Find` 一种建筑，没有才 `Build`。座位写在名字末尾（`5`、`6`、`P5`、`P6`）。

| 脚本 | 作用 |
|---|---|
| `Build Barracks`、`Build P Barracks`、`SovietBarracks`、`JapanBarracks`、`AlliedBarracks` | 兵营 |
| `Build P WarFactory`、`Build Units WarFactory`、`Build Units WarFactory5`、`JapanWarFactory`、`Build JapanWarFactory` | 重工。末尾 `N3` 是第三座 |
| `JapanNavalYard`、`SovietNavalYard1`、`SovietNavalYard2`、`UNIT - Allies 02 Naval Yard` | 船厂。海图上几乎都有 |
| `AlliedAirfield`、`Air Force` | 机场、空军 |
| `Build Refinery`、`Build Refinery5`、`Build Refinery6` | 矿场，按座位各一座 |
| `Build Tech`、`Build Tech Disable`、`Build SuperWeapon` | 高科和超级武器。`Disable` 是先关掉，到点再允许 |
| `Allied Outpost5 Tech2`、`Allied Outpost5 Tech3`、`Find AlliedOutpost5` | 前哨，T2、T3 各一档 |
| `Build P JapanWarFactoryEgg`、`Build P Barracks Openmove`、`Build Units Openmove` | 蛋车或可移动建造。`Openmove` 是展开后允许移动 |
| `AI Buff0`、`AI Buff1`、`AI Buff2` | 电脑增益三档，0 最前 |
| `Allied Buff5`、`Allied Buff6`、`Japan Buff5`、`Japan Buff6` | 只给 P5 或 P6 的该阵营 |

`Find Japan Units` 在造完之后清点部队，供 `Hunt` 使用。

### 5. 进攻

`Hunt` 出现在 8 张图的公共脚本里，是 SMAI 的出击入口，不按波次拆开。和它配套的是 `Find Japan Units`、`Air Force`。

easter island 另有 `Tesla Tank Unlock`、`Timer - Tesla Tank`、`P WarFactory - Spawn`、`P PowerPlant - Spawn`，是磁能坦克和重工、电厂的生成，只属于这张图。

### 6. 计时

| 脚本 | 作用 |
|---|---|
| `TIMER GAME START`、`Game Start Timer Set` | 开局计时起点 |
| `TIMER GAME START ADD`、`Game Start Timer Add`、`Timer - Add` | 往后加一段，用来错开第二次给钱或第二次建造 |
| `Timer - Stop` | 停掉这段计时 |

### 7. 外交

`Relation` 在 8 张图里单独成组，作用和 NKL 的 `AI Relationship` 相同：排座位敌友。`Team`、`R - TEAM - Allies 04 Spawn` 是队伍生成，不是改外交；mykonos 用它刷盟军小队。

### 8. 人类侧

SMAI 很少给人类单独写脚本。名字里的 `Buff5`、`Buff6` 挂在电脑座位上。`Plyer` 组里的 `PMoney` 是给某个玩家的钱，和 `AI Money` 并列，只在两张图里出现。

### 9. 单图事件

| 图或组 | 脚本 | 作用 |
|---|---|---|
| 多张 | `NR_Unit0`、`NR_Unit1`、`NR_1`、`NR_2` | 中立单位或地图规则单位 |
| 两张 | `Camera0`–`Camera3` | 镜头 |
| sigma 等 | `ObTower`、任务、旗帜 | 地图目标建筑，和 `AI` 组并列 |
| mykonos | `Nel_Allies Base`、`Base - A A01`、`TEAM - Allies 04 Spawn` | 预先摆好的盟军基地和刷出的小队 |
| 两张里的 `X` 组 | `Show Objective`、`Autosave`、`Play Engineer Audio`、`Activate Hotspot` | 战役地图残留，不是遭遇战 AI |

`R.D`、`Rarda` 的 `R1`–`R3` 是随机分支，和 NKL 的 `R1`/`R2` 同类。

### 分图索引

条数是该家族里脚本较多的一版。版本之间差几条到几十条，组结构不变。

| 图 | 条数 | 用了哪些类 |
|---|---|---|
| massacre valley t1.1 / t1.5 | 58–62 | 钱、`AI Skill Lock`、雷达、开局计时；日/苏/盟按 P5–P8 找厂再补 |
| easter island / t1.4 | 51 | 帝国与苏联的厂、雷达、计时、外交；另有磁能坦克解锁 |
| free port / t1.3 | 48 | `AI MONEY`、盟军与苏联、雷达、计时、BGM |
| joint operations t1.1 | 51 | 4v4 组里的雷达和 AI，加 BGM |
| hidden fortress | 37 | 雷达、音乐、盟军与帝国、外交。三系里 SMAI 最短的一版 |
| barents sea | 68 | 帝国与盟军，按玩家分支 |
| island adventure remake | 71 | 三阵营 AI |
| loch mess | 70 | 开局时间、敌友、全图、盟军与苏联 |
| war strait | 77 | 任务、刷兵、雷达；AI 组分成两支 |
| silent hill | 76 | 三阵营、随机、外交、开局计时；另有磁能与精英单位 |
| battlebase sigma / v1.07 | 119–123 | 计时、敌对、三阵营 AI，加中立单位 |
| fallout t1.0 / t1.1 | 130–132 | 巡逻、`AI ATK`、开局兵营、`Money SP`、三阵营 |
| unreal city t1.0 | 132 | `AI Money`、三阵营；另有幻影坦克、海豚巡逻、任务塔 |
| snow burial t1.9 | 135 | `Money SP`、三阵营；另有塔和蜘蛛 |
| polar execution t1.82 | 139 | `SkirmishAI`、`Map AI Money`、苏联与盟军；另有机甲和防御血量 |
| mykonos / v1.02 | 201–246 | 公共列表加上 Civilian、Creeps。盟友基地和刷兵是这张图自己的；解析不完整 |

## AXAI

45 张图。发育不是「缺了补一座」，而是每个生产建筑一条生命周期，同一套名字在 30 张以上的图里重复。公共列表放启动、阵营开关和这张图的玩法；生命周期挂在该强的座位自己的列表上。人类座位多半只有奖励。

入门麦格马戈登只有约 52 条，是这套的短版：公共列表上的 `MONEY` 和苏/盟 `AI`，六个玩家位几乎是空的。死亡搁浅把完整生命周期铺进 `Player_1` 到 `Player_6`，所以可以单机。巴顿码头的船厂、重工、兵营生命周期在 `PlyrCreeps` 上。

`FE` 在 45 张的脚本根上，和 `FIND`、开局计时放在一起，是整包的启动入口。

### 1. 开场

| 脚本 | 作用 |
|---|---|
| `FE` | 启动入口 |
| `Time-Game Start up`、`Time_Game Start`、`时间` | 开局计时，之后交给各个厂自己的 `T2TIME` |
| `obj` | 任务目标开关 |
| `music`、`music2` | 音乐。只有少数图有 |
| `你的名字` 下的 `休闲~打电脑`、`打电脑2`、`作者` | 说明文字或难度标签，不是生产 |

### 2. 钱和开矿

钱和矿场绑在一起，不按 `Soviet1`–`Soviet5` 这种空拍发放。

| 脚本 | 作用 |
|---|---|
| `money`、`money11` | 发钱。`MONEY` 组里的 `P/6`、`P/8` 是按座位发 |
| `开矿`、`开矿1`、`开矿2`、`开矿s` | 第几座矿。`开矿s` 在苏联组 |
| `开矿展开`、`开矿展开2` | 矿区从第一圈扩到下一圈 |
| `开矿展开1关闭`、`开矿展开2关闭` | 这一圈扩完就关掉，避免重复下指令 |
| `开矿矿场` | 矿场建筑本身 |
| `ai电力`、`补充电力`、`大电` | 电厂。矿场多了就补电 |

`开矿` / `开矿展开` / `关闭` 这三步在 41 张以上的图里成套出现。

### 3. 阵营和基地车

| 脚本 | 作用 |
|---|---|
| `帝国启用`、`盟军启用`、`苏联启用` | 三选一。座位是哪个阵营就打开哪一支，另外两支不跑。37 张图有 |
| `Japan MCV On`、`Soviet MCV On`、`帝国MCV`、`PlyrCivilianMCV On` | 打开该阵营基地车 |
| `MCV t2`、`MCV t3` | 基地车升到对应科技后再允许下一步 |
| `MCV删除`、`是否为苏联` | 删掉与阵营不符的基地车。麦格马戈登的 `苏联mcv` 组用这个 |
| `FIND` 下的 `帝国兵营1`、`帝国重工1`、`帝国船厂1`、`帝国船厂2`、`帝国船厂3` | 先在地图上找到已经存在的厂，再决定造不造 |

`A5 Delete JS`、`A6 Delete JS`、`S6 Delete AJ` 在 `AI Hunting Script` 里：这个座位若是盟军，就删掉预放的帝国和苏联建筑；若是苏联，就删掉盟军和帝国。和 NKL 的 `remove 5 s mcv` 是同一件事，AXAI 写成按座位的删除。

### 4. 发育：一座厂的生命周期

兵营、重工、船厂各用同一套后缀。`1`、`2` 是第几座，不是科技等级。科技等级写在 `T2`、`T3` 里。下面用兵营代表，重工、船厂把「兵营」换成自己的名字即可。船厂这套在 15 张以上的图里出现，重工和兵营在 26 张以上。

| 后缀 | 作用 |
|---|---|
| `兵营1-t1建造`、`兵营1-t2建造`、`兵营1-t3建造` | 在对应科技下单造这座兵营 |
| `兵营1-T2`、`兵营1T2TIME`、`兵营T2激活` | 计时到了才允许 T2，激活是真正打开生产 |
| `兵营1-关闭T2`、`兵营1-关闭T3` | 离开该时间窗后关掉，防止一直重复下单 |
| `兵营1摧毁判定`、`兵营2丢失判定` | 被打掉或从名单里消失之后，回到建造步 |
| `工厂1`、`工厂2`、`工厂切换` | 这座厂当前生产哪一类。`切换` 在 T2、T3 之间换生产项目 |
| `机场1`、`机场2`、`机场1建造`、`机场2建造`、`苏联机场` | 机场。苏联组里的机场和通用机场是同一类建筑的两套名字 |
| `高科`、`苏联高科`、`高科丢失判定` | 科技中心。丢了就重判 |
| `防御塔`、`高级防御塔` | 基础塔和升级塔。37 张图有 |
| `升级1`、`升级2` | 帝国线上的升级步，和 `直美1`、`直美2` 写在同一组 |
| `直美1`、`直美2` | 帝国生产线上的一种单位，和升级放在一起 |
| `造维和1`、`造维和2` | 盟军维和步兵，两座厂或两拍 |
| `开启 造兵_`、`开启 机场_`、`工厂__牛蛙`、`苏联船厂建造牛蛙` | 民用位或苏联线上的具体生产开关 |

盟军组里的固定成员是 `机场1`、`开矿矿场`、`MCV t2`、`MCV t3`、`开矿展开2`、`造维和`。苏联组是 `开矿1`、`苏联机场`、`苏联_War`。帝国组是 `高科`、`升级`、`开矿`、`直美`。`Pr Japan` 是帝国模板的另一份拷贝，带 `Japan MCV On`、`补充电力`、兵营和重工的丢失判定。

`PlyrCivilianAI` 上再放一份苏联向的厂：`大电`、`机场`、`苏联兵营`、`工厂__牛蛙`。终末阵地、古城危机的额外位用的就是这份。

### 5. 进攻

| 脚本 | 作用 |
|---|---|
| `造兵`、`造兵1`、`造兵2` | 按拍出兵。出兵本身就是进攻的来源，多数图没有单独的 `Hunt` |
| `苏联_War` | 苏联组里的出击 |
| `AI Hunting Script` 下的 `ST2`、`S52`、`A52`、`A62`、`S61` | 终末阵地这一族的狩猎。字母是阵营，数字是座位 |
| `对位陆军`、`对位海军处理`、`对位上岛处理` | 按对手在陆地、海上还是需要登陆来选打法。巴顿码头、终末阵地的 Creeps 列表 |
| `5分钟加强`、`10分钟加强`、`20分钟加强` | 玩法修补组里的定时加强，只在少数图 |

### 6. 计时

除了开局的 `Time_Game Start`，每个厂用自己的 `兵营1T2TIME`、`重工1T3TIME`、`船厂1T2TIME` 决定何时激活、何时关闭。`关闭t2` 是全局的 T2 窗口结束。

### 7. 外交

AXAI 没有 NKL、SMAI 那种 `Relationship` 组。敌友用房间队伍。脚本里和阵营有关的删除（`Delete JS`）是清预放建筑，不是改外交。

### 8. 人类侧和奖励

挂在人类座位的列表上，不挂在电脑的生命周期里。

| 脚本 | 作用 |
|---|---|
| `杀敌计数器`、`比较计数器`、`激活杀敌赏金`、`转换` | 杀敌累计到一档就发一笔钱。`转换` 是计数器进位 |
| `哨站1占领`、`哨站1摧毁`、`哨站2占领`、`任务开始` | 哨站任务 |
| `奖励` 组里的 `电站` | 给人类座位一座电厂或一笔电相关奖励。遗忘之地的 P1–P4 主要是这个 |
| `失败` | 奖励组里的失败条件 |

难度只在遗忘之地写成脚本：`难度投票`、`难度选择脚本`，电脑座位上再分 `困难` 和 `中等`，每档下面是帝、盟、苏三支。其余 AXAI 图不看房间难度。`针对训练脚本` 是训练版多出来的一层，标准版没有。

### 9. 单图事件

| 组 | 脚本 | 作用 |
|---|---|---|
| `BOSS` | `BOSS ON`、`死亡`、`被缩小`、`镰刀机甲回血`、`镰刀启动减cd`、`投毒cd结束判断` | 麦格马戈登的头目战 |
| `蜘蛛` | `蜘蛛`、`蜘蛛回血`、`蜘蛛死亡`、`回复技能` | 可击杀的地图生物 |
| `蜂巢` | `蜂巢判断`、`蜂巢使用` | 帝国蜂巢 |
| `毁灭装置` | `判断毁灭装置`、`使用毁灭装置` | 超级武器式的地图装置 |
| `英雄随机判断脚本`、`随机数不重复` | `P1大于`–`P4大于`、`无相等数结束循环` | 给玩家随机一个不重复的结果 |
| `协议统计` | `决胜中队1级`、`冷冻卫星2级`、`海军协议`、`毒质腐蚀`、`时空炸弹1级`、`气球炸弹3级`、`帽子协议` | 统计或发放各级协议。古城危机娱乐版、黄金树之影按玩家列表各放一份，所以脚本数到 1000 左右 |
| `全图__1`、`全图__2` | `全图视野` | 开全图视野，两份是两个阵营或两侧 |
| `玩法修补` | `机场消失`、`兵营消失`、`重工消失`、`英雄消失`、`基洛夫自爆开始` | 修这张图的特例：某种建筑到点消失，基洛夫自爆 |
| `房屋修复` | `房屋修复`、`房屋修复开始2`、`房屋修复停止2`、`房屋修复保护` | 修房子的小玩法 |
| `帝国防空` | `允许建造防空塔`、`禁止建造其他`、`任务目标完成` | 麦格马戈登的一项限制任务 |
| `废弃脚本` | 与正式生命周期同名的旧拷贝 | 留在图里但不作为现行逻辑 |

### 分图索引

条数取该家族里较多的一版。娱乐版和黄金树到 1000 条，是因为协议、英雄、奖励按玩家复制，生命周期本身仍是上面那一套。

| 图 | 条数 | 强 AI 挂在谁身上 | 公共列表多出来的玩法 |
|---|---|---|---|
| 入门麦格马戈登 | 52 | 只在公共列表 | 计时、钱、苏/盟 AI |
| 麦格马戈登、新麦格马戈登 v1.1–v1.4 | 305–388 | 公共列表的苏/盟 AI；玩家位上是蜘蛛、基地车、防空 | BOSS、镰刀机甲、杀敌赏金、随机数 |
| 巴顿码头（含 v1.21、测试版） | 231–253 | `PlyrCreeps` 上的船厂、重工、兵营生命周期 | 苏/日建造模板、对位海陆 |
| 遗忘之地测试版、测试版 2/3 | 319–343 | `Player_5`、`Player_6` 按帝/盟/苏建造和开矿 | 奖励、恐怖机器人 |
| 遗忘之地测试版 4.1 | 368 | 同上，并分成困难、中等 | 难度投票、难度选择 |
| 遗忘之地 v1.01 / v1.02 | 613 | 生产脚本更厚 | 千人斩、称号 |
| 古城危机 xex、三国、ancient city crisis | 155–268 | 相关玩家位上的开矿、机场、盟军建造；民用位另有苏/日建造 | 针对训练版多一组训练脚本 |
| 古城危机娱乐版、黄金树之影（含针对训练） | 1000 上下 | 每个相关玩家列表都有兵营、重工、机场、协议统计 | 英雄随机、鬼王、基洛夫、超级武器协议 |
| 终末阵地（含 t1.2、t1.4、t1.10、v1.0） | 399–458 | `Player_5`、`Player_6` 和 `PlyrCreeps` | 守点、失败条件、杀敌返金；后期版本加相邻加强和中路开矿 |
| 死亡搁浅（可单机） | 791 | `Player_1` 到 `Player_6` 都有完整脚本 | 杀敌赏金、公海数量、T3 战舰支援 |
| Carville 6v6 | 134 | 多个建设位；Creeps 上是兵营、重工、机场，另一额外位是船厂 | 游戏结束判定 |
