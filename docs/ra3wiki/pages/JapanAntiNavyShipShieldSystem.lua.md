---
title: "JapanAntiNavyShipShieldSystem.lua"
catalog_path: "Lua例子/【日冕】太刀新弹射护盾的相关 Lua 代码"
slug: bq16e0hre6fypi3t
source: https://www.yuque.com/muzeqaq/ra3mapwiki/bq16e0hre6fypi3t
fetched: 2026-10-15
content_updated_at: "2024-09-30T04:08:23.000Z"
type: Doc
format: lake
---

# JapanAntiNavyShipShieldSystem.lua

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/bq16e0hre6fypi3t
> 目录位置：Lua例子/【日冕】太刀新弹射护盾的相关 Lua 代码

```lua
-- 以下文件是太刀新弹射护盾的相关 Lua 代码。
-- 日冕内部源码，如假包换。
-- 实际上，光凭 Lua 代码是无法实现这个弹射护盾的，需要与日冕其他部分的配合才能做到。
-- 假如只仅仅阅读这个文件，大概也很难理解这个弹射护盾是如何工作的。
-- 但是这并不重要！
-- 这个文件作为一个范例，来让大家了解 Lua 的语法以及各种函数的用法，就足够了！

-- RunRA3Lua 这个函数在地图编辑器里无法使用，
-- 在地图编辑器里也不能通过 RunRA3Lua 来加载更多的 Lua 文件，
-- 不过为了方便阅读，这里还是保留了这个函数的调用。
-- LinkedList.lua 这个文件也加到压缩包里了。
RunRA3Lua("Data\\Scripts\\LinkedList.lua")

-- 红警3的武器本身不能做到计算提前量，也不能避免在同一个区域内重复创建护盾
-- 利用 Lua 脚本可以做到这些，但是在 Lua 脚本里模拟武器的开火间隔和装弹时间又很麻烦

-- 因此，这里采用了一个混合的方法：
-- 由 Lua 来控制护盾的创建，但是开火间隔和装弹时间仍然由武器自己控制

-- 因此，武器只是用来为太刀的弹射护盾系统控制开火间隔和装弹时间的
-- 并不会实际创建护盾

-- Lua 脚本在发现合适的目标之后，将会创建护盾。
-- 同时也将命令 ShieldSystem 攻击，这将导致假武器开火，
-- 从而设置开火间隔和装弹时间相关的状态。
-- Lua 脚本在这些状态启用的时候不会再次创建护盾。
-- 但是武器开火有至少一帧的延迟，
-- 为了解决这个问题，Lua 在创建护盾时也会另外设置一个状态，临时阻止后续的护盾创建。
-- 当 Lua 脚本检测到武器相关的状态之后，会取消临时状态，改为用武器状态来判定开火间隔和装弹时间。

JapanAntiNavyShipShieldSystem = {
    -- 护盾系统的最大索敌距离。在这个距离以内出现的敌方抛射体都会被考虑
    MAX_ENEMY_PROJECTILE_DETECTION_DISTANCE = 400,
    -- 护盾的最大生成距离
    MAX_SHIELD_PROTECTION_DISTANCE = 300,
    FRAMES_TO_PREDICT = 3, -- 根据敌方抛射体在 N 帧之后的位置来计算护盾的位置
    -- DEPLOY_DISTANCE 是护盾展开之前，额外向前移动的距离。
    -- 但实际上，DEPLOY_DISTANCE 并不会实际控制护盾的展开距离，它只是用于计算生成护盾的位置而已。
    -- 实际的移动距离是由护盾的 <ProjectileUpdate /> 控制的。
    DEPLOY_DISTANCE = 65,
    -- <ProjectileUpdate /> 抵达终点后会直接摧毁护盾
    -- 所以护盾的终点得设在一个方向相同但是距离更远的位置
    EXTRA_DISTANCE = 500,

    SHIELD_DISTANCE_GAP = 75, -- 护盾生成的最小间隔

    VELOCITY_EPSILON = 0.0625, -- 速度的低于这个值的抛射体不会被考虑
    -- 从天而降的护盾自上而下拦截垂发导弹过于诡异。
    -- 因此，单位化速度之后 Z 轴大于这个值的抛射体会被视作垂发导弹，暂时不会被护盾拦截
    VERTICAL_PROJECTILE_THRESHOLD = 0.6,
    VECTOR_CROSS_THRESHOLD = 0.9375, -- 单位化速度向量之后 Z 轴高于这个值的向量将会被特别处理

    -- 根据实测测出来的护盾碰撞盒大小
    SHIELD_SIZE = {
        FRONT = 10.0,
        BACK = -5.0,
        LEFT = -30,
        RIGHT = 30,
        TOP = 25,
        BOTTOM = -35,
    },
    SHIELD_COLLISION_DETECTION_DISTANCE = 150,

    -- 护盾每帧损失血量
    SHIELD_HIGH_DAMAGE_PER_FRAME = 300 / 15, -- 没有被充能的时候，弹射护盾每帧损失大量血量
    SHIELD_LOW_DAMAGE_PER_FRAME = 75 / 15, -- 被充能的时候，弹射护盾每帧损失少量血量

    -- 护盾随血量改变阵营色
    -- 懒得画贴图来表示护盾的颜色了，直接用 Lua 改（
    SHIELD_COLOR = { 0, 160, 225 },
    SHIELD_DAMAGED_COLOR = { 225, 128, 0 },
    SHIELD_CRITICAL_COLOR = { 255, 24, 0 },
    SHIELD_DESTROYED_COLOR = { 128, 128, 128 },

    -- 临时禁止创建护盾（在武器状态接管开火间隔和装弹时间之前使用）
    TEMPORARY_NO_SHIELD_STATUS = "GENERIC_TOGGLE_STATE",

    -- 创建弹射护盾系统
    ShieldSystemName = "JapanAntiNavyShipTech3ShieldSystem",
    CreateShieldSystemWeapon = "CreateJapanAntiNavyShipTech3ShieldSystem",
    -- 创建以及发射弹射护盾
    ShieldCreationHelperName = "JapanShieldProjectileCreationHelper",
    CreateShieldProjectileWeapon = "CreateJapanShieldProjectile",
    -- 用于检测太刀是否已经拥有弹射护盾系统
    LinkShieldSystemFilter = CreateObjectFilter({
        Relationship = "SAME_PLAYER",
        IncludeThing = { "JapanAntiNavyShipTech3ShieldSystem" }
    }),
    -- 太刀以下瘫痪状态，禁用护盾系统
    -- （其实就是除了 HELD 允许执行 Lua 之外，其他的都不允许）
    ShipCannotLaunchShieldDisabledTypes = ParseDisabledBitFlags(
        "DEFAULT USER_PARALYZED EMP"
        .. " PARALYZED UNMANNED UNDERPOWERED FREEFALL TEMPORARILY_BUSY"
        .. " SCRIPT_DISABLED SCRIPT_UNDERPOWERED INFILTRATED"
    ),
    -- 弹射护盾系统是否正在开火或处于开火间隔内
    ShieldSystemFiringFilter = CreateObjectFilter({
        Rule = "ALL",
        StatusBitFlags = "IS_FIRING_WEAPON",
    }),
    -- 弹射护盾系统是否正在装填
    ShieldSystemReloadingFilter = CreateObjectFilter({
        Rule = "ALL",
        StatusBitFlags = "IS_RELOADING_WEAPON",
    }),
    -- 用于识别太刀弹射护盾子炮塔
    RechargeShieldTurretFilter = CreateObjectFilter({
        Relationship = "SAME_PLAYER",
        IncludeThing = { "JapanShipShieldTurret" }
    }),
    -- 用于检测某个区域内是否已经存在护盾（避免重复创建护盾）
    DuplicateShieldFilter = CreateObjectFilter({
        Relationship = "SAME_PLAYER ALLIES",
        IncludeThing = {
            "JapanShieldProjectile"
        }
    }),
    -- 防止与友军重叠的 Filter
    OverlapWithAlliesFilter = CreateObjectFilter({
        Rule = "ANY",
        Include = "SCORE",
        StatusBitFlagsExclude = "SUBMERGED",
    }),
    -- 敌方抛射体 Filter
    EnemyProjectileFilter = CreateObjectFilter({
        Relationship = "ENEMIES",
        Rule = "ANY",
        Include = "SMALL_MISSILE BALLISTIC_MISSILE",
        Exclude="COMBO_HORDE THROWN_OBJECT SUBMARINE",
        StatusBitFlagsExclude="SUBMERGED UNDER_IRON_CURTAIN",
        -- 以下导弹不能被拦截
        ExcludeThing = {
            -- 波塞冬炮弹
            "AlliedGunshipAircraftWeaponProjectile",
            "AlliedGunshipAircraftWeaponProjectile_Veteran",
            -- 航母 EMP
            "AlliedAntiStructureShipEMPProjectile",
            -- 冰山导弹
            "AlliedSupportAircraftIceMissileProjectile",
            "AlliedAntiNavyShipTech3IceMissileProjectile",
        }
    }),
    -- 护盾是否正在被充能
    ShieldChargedFilter = CreateObjectFilter({
        Rule = "ALL",
        StatusBitFlags = "OVERCHARGING_WEAPON",
    }),
    NewShieldListByPlayerName = {}, -- 这一帧新生成的护盾，用于确保不会在同一帧内在相同的位置生成多个护盾
    NewShieldsDataLastClearFrame = -1,

    -- 护盾系统列表
    ShieldSystemList = LinkedList(),
    -- 护盾列表
    ShieldList = LinkedList(),
}

function JapanAntiNavyShipTech3_EnsureShieldSystem(ship)
    -- 确保太刀的子炮塔能够给友方护盾充能
    local rechargeShieldTurretFilter = JapanAntiNavyShipShieldSystem.RechargeShieldTurretFilter
    local table, count = ObjectGetContainedPassengers(ship)
    for i = 1, count, 1 do
        local turret = table[i]
        if ObjectTestTargetObjectWithFilter(ship, turret, rechargeShieldTurretFilter) then
            if not EvaluateCondition("UNIT_HAS_OBJECT_STATUS", turret, "REPAIR_ALLIES_WHEN_IDLE") then
                ObjectSetObjectStatus(turret, "REPAIR_ALLIES_WHEN_IDLE")
            end
        end
    end

    -- 确保太刀有一个弹射护盾系统
    local shieldSystemFilter = JapanAntiNavyShipShieldSystem.LinkShieldSystemFilter
    local shieldSystemUnit = nil
    local attachers, attachersCount = ObjectGetAttachers(ship)
    for i = 1, attachersCount, 1 do
        local attacher = attachers[i]
        if ObjectTestTargetObjectWithFilter(ship, attacher, shieldSystemFilter) then
            shieldSystemUnit = attacher
            break
        end
    end
    if shieldSystemUnit ~= nil then
        -- 一切正常
        return
    end
    -- 没有弹射护盾系统，或者弹射护盾系统已经被摧毁了
    -- 创建一个新的弹射护盾系统
    ObjectCreateAndFireTempWeapon(ship, JapanAntiNavyShipShieldSystem.CreateShieldSystemWeapon)
end

function JapanAntiNavyShipTech3_ShieldSystem_OnCreated(shieldSystemUnit)
    JapanAntiNavyShipShieldSystem.ShieldSystemList:AppendNode(shieldSystemUnit)
end

function JapanAntiNavyShipTech3_ShieldSystem_OnHelperCreated(helperUnit)
    -- 获取关联信息
    local playerScriptName = ObjectPlayerScriptName(helperUnit)
    if playerScriptName == nil then
        -- 假如没有归属玩家，表明是在加载存档阶段，或者自定义地图用脚本强行刷了一个出来
        -- 这种情况下，不需要关联信息
        return
    end
    local newShieldList = JapanAntiNavyShipShieldSystem.NewShieldListByPlayerName[playerScriptName]
    if newShieldList == nil then
        -- 没有找到关联信息，这就坏了
        return
    end
    local lastShieldData = newShieldList.Previous
    if lastShieldData == newShieldList.Anchor then
        -- 没有找到关联信息，这就坏了
        return
    end
    if lastShieldData.HelperUnit ~= nil then
        -- 已经关联过了，这就坏了
        return
    end
    lastShieldData.HelperUnit = helperUnit
    -- JapanAntiNavyShipShieldSystem.CreateNewShield 在得到 helperUnit 之后将会继续执行
end

function JapanAntiNavyShipTech3_ShieldSystem_OnShieldCreated(shieldUnit)
    JapanAntiNavyShipShieldSystem.ShieldList:AppendNode(shieldUnit)

    -- 获取关联信息
    local playerScriptName = ObjectPlayerScriptName(shieldUnit)
    if playerScriptName == nil then
        -- 假如没有归属玩家，表明是在加载存档阶段，或者自定义地图用脚本强行刷了一个护盾出来
        -- 这种情况下，不需要关联信息
        return
    end
    local newShieldList = JapanAntiNavyShipShieldSystem.NewShieldListByPlayerName[playerScriptName]
    if newShieldList == nil then
        -- 没有找到关联信息，这就坏了
        return
    end
    local lastShieldData = newShieldList.Previous
    if lastShieldData == newShieldList.Anchor then
        -- 没有找到关联信息，这就坏了
        return
    end
    if lastShieldData.ShieldUnit ~= nil then
        -- 已经关联过了，这就坏了
        return
    end
    lastShieldData.ShieldUnit = shieldUnit
    -- JapanAntiNavyShipShieldSystem.CreateNewShield 在得到 shieldUnit 之后将会继续执行
end

function JapanAntiNavyShipTech3_ShieldSystem_OnShieldDestroyed(shieldUnit)
    -- 让护盾变成死亡颜色
    JapanAntiNavyShipShieldSystem:UpdateShieldColor(shieldUnit)
end

-- 由 CoronaLuaRunner 每帧调用
function JapanAntiNavyShipShieldSystem.UpdateAll(system)
    -- 更新所有护盾系统
    local anchor = system.ShieldSystemList.Anchor
    local node = anchor.Next
    while node ~= anchor do
        local nextNode = node.Next
        if ObjectIsAlive(node) then
            system:ShieldSystemUpdate(node)
        else
            -- 从链表中删除这个已经不再存在的单位
            system.ShieldSystemList:RemoveNode(node)
        end
        node = nextNode
    end
    -- 更新所有弹射护盾
    anchor = system.ShieldList.Anchor
    node = anchor.Next
    while node ~= anchor do
        local nextNode = node.Next
        if ObjectIsAlive(node) then
            system:ShieldUpdate(node)
        else
            -- 从链表中删除这个已经不再存在的单位
            system.ShieldList:RemoveNode(node)
        end
        node = nextNode
    end
end

-- 自动分弹（抄袭哨机自动分弹代码）
function JapanAntiNavyShipTech3_ShieldTurret_OnFiringB(shieldTurret) end
function JapanAntiNavyShipTech3_ShieldTurret_OnNotFiringB(shieldTurret) ObjectRemoveUpgrade(shieldTurret, "Upgrade_NotFirstShoot") end
function JapanAntiNavyShipTech3_ShieldTurret_OnUser10(shieldTurret) ObjectGrantUpgrade(shieldTurret, "Upgrade_NotFirstShoot") end

function JapanAntiNavyShipShieldSystem.ShieldSystemUpdate(system, shieldSystemUnit)
    -- 确保弹射护盾系统与太刀关联
    local ship = system:EnsureShieldSystemLinkedToShip(shieldSystemUnit)
    if ship == nil then
        return
    end

    -- 删除旧数据
    system:ClearOldData()

    if ObjectIsDisabled(ship, system.ShipCannotLaunchShieldDisabledTypes) then
        if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS) then
            -- 确保假武器不在开火
            ExecuteAction("NAMED_STOP", shieldSystemUnit)
        end
        -- 禁止发射护盾
        return
    end

    if ObjectTestTargetObjectWithFilter(nil, shieldSystemUnit, system.ShieldSystemReloadingFilter) then
        if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS) then
            -- 已经有武器状态了，不需要再使用临时状态来避免创建护盾了
            ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS, 0)
            -- 确保假武器只开火一次、不能再次开火，否则开火间隔和装弹的状态会永远存在
            ExecuteAction("NAMED_STOP", shieldSystemUnit)
        end
        -- 尚未装填完毕
        return
    end

    if ObjectTestTargetObjectWithFilter(nil, shieldSystemUnit, system.ShieldSystemFiringFilter) then
        if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS) then
            -- 已经有武器状态了，不需要再使用临时状态来避免创建护盾了
            ExecuteAction("UNIT_CHANGE_OBJECT_STATUS", shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS, 0)
            -- 确保假武器只开火一次、不能再次开火，否则开火间隔和装弹的状态会永远存在
            ExecuteAction("NAMED_STOP", shieldSystemUnit)
        end
        -- 正处于开火间隔中
        return
    end

    -- 假如武器不在开火间隔中，也不在装填中，但是却有临时禁止创建护盾的状态
    -- 那很可能是前面一帧创建了护盾之后，让护盾系统攻击并启用假武器的操作失败了
    if EvaluateCondition("UNIT_HAS_OBJECT_STATUS", shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS) then
        -- 让我们再试一次
        ExecuteAction("NAMED_FORCE_ATTACK_NAMED", shieldSystemUnit, ship)
        return
    end

    -- 获取护盾目标
    local shieldPosition = system:FindPosition(ship, shieldSystemUnit)
    if shieldPosition == nil then
        ExecuteAction("NAMED_STOP", shieldSystemUnit)
        return
    end

    -- 创建护盾
    system:CreateNewShield(shieldSystemUnit, shieldPosition)
    -- 禁止之后再次开火（除非被武器的开火时间 / 装弹时间接管）
    ObjectSetObjectStatus(shieldSystemUnit, system.TEMPORARY_NO_SHIELD_STATUS)
    -- 命令攻击，从而启用假武器，从而能利用武器状态来判断开火间隔和装弹时间
    ExecuteAction("NAMED_FORCE_ATTACK_NAMED", shieldSystemUnit, ship)
end

function JapanAntiNavyShipShieldSystem.EnsureShieldSystemLinkedToShip(system, shieldSystemUnit)
    -- 确保弹射护盾系统与太刀关联，成功返回关联的太刀，失败返回 nil
    local ship = ObjectGetAttachees(shieldSystemUnit)[1]
    if ship ~= nil then
        -- 已经是成功关联状态
        -- 检查玩家
        if ObjectPlayerScriptName(ship) == ObjectPlayerScriptName(shieldSystemUnit) then
            -- 玩家相同，检查太刀是否依然存活 
            if ObjectIsAlive(ship) then
                return ship
            end
        end
    end

    -- 取消关联（太刀会自行生成新的弹射护盾系统）
    -- 销毁自身
    ExecuteAction("NAMED_DELETE", shieldSystemUnit)
    return nil
end

function JapanAntiNavyShipShieldSystem.ClearOldData(system)
    local currentFrameNumber = GetFrame()
    if system.NewShieldsDataLastClearFrame >= currentFrameNumber then
        return
    end
    system.NewShieldsDataLastClearFrame = currentFrameNumber
    -- 删除无效的数据
    -- 这个操作不会修改任何实际上的游戏数据，
    -- 而且无论使用哪种顺序来遍历 table，最终各个 table 都一样会被清空
    -- 因此所以可以使用 for key, value in table 的方式，
    -- 而不用担心遍历顺序未知而导致游戏数据不同步
    for playerName, newShieldList in system.NewShieldListByPlayerName do
        local anchor = newShieldList.Anchor
        local node = anchor.Next
        while node ~= anchor do
            local nextNode = node.Next
            -- 老旧的护盾
            if currentFrameNumber - node.FrameNumber > 5 then
                newShieldList:RemoveNode(node)
            end
            node = nextNode
        end
    end
end

function JapanAntiNavyShipShieldSystem.FindPosition(system, ship, shieldSystemUnit)
    local playerScriptName = ObjectPlayerScriptName(shieldSystemUnit)
    local newShieldList = system.NewShieldListByPlayerName[playerScriptName]
    if newShieldList == nil then
        newShieldList = LinkedList()
        system.NewShieldListByPlayerName[playerScriptName] = newShieldList
    end

    -- 获取太刀一定范围内的所有需要找到的抛射体
    local x, y, z = ObjectGetPosition(shieldSystemUnit)
    local targets, targetCount = ObjectFindObjects(shieldSystemUnit, {
        X = x, Y = y, Z = z,
        Radius = system.MAX_ENEMY_PROJECTILE_DETECTION_DISTANCE,
        DistType = "SPHERE"
    }, system.EnemyProjectileFilter)
    -- 处理每一个找到的抛射体
    for i = 1, targetCount, 1 do
        local position = system:CheckPosition(ship, targets[i], newShieldList)
        if position ~= nil then
            return position
        end
    end
    return nil
end

function JapanAntiNavyShipShieldSystem.CheckPosition(system, ship, enemyObject, newShieldList)
    local enemyX, enemyY, enemyZ = ObjectGetPosition(enemyObject)
    local enemyPx, enemyPy, enemyPz = ObjectGetPreviousPosition(enemyObject)
    local enemyVx, enemyVy, enemyVz = enemyX - enemyPx, enemyY - enemyPy, enemyZ - enemyPz
    local enemySpeed = sqrt(enemyVx * enemyVx + enemyVy * enemyVy + enemyVz * enemyVz)
    if enemySpeed < system.VELOCITY_EPSILON then
        -- 速度太小，无法计算朝向
        -- 按理说不可能出现这种情况，不过，万一真的出现了，会导致游戏崩溃，所以还是加上这个判断，保险一点（
        return nil
    end
    if enemyVz / enemySpeed > system.VERTICAL_PROJECTILE_THRESHOLD then
        -- 速度向量的 Z 轴太大，这个抛射体是垂发导弹，不考虑
        return nil
    end

    local framesToPredict = system.FRAMES_TO_PREDICT
    local squaredMaxShieldProtectionDistance = system.MAX_SHIELD_PROTECTION_DISTANCE
    squaredMaxShieldProtectionDistance = squaredMaxShieldProtectionDistance * squaredMaxShieldProtectionDistance
    local squaredShieldDistanceGap = system.SHIELD_DISTANCE_GAP
    squaredShieldDistanceGap = squaredShieldDistanceGap * squaredShieldDistanceGap

    -- 预测的坐标
    local nx = enemyX + enemyVx * framesToPredict
    local ny = enemyY + enemyVy * framesToPredict
    local nz = enemyZ + enemyVz * framesToPredict
    -- 船的坐标
    local sx, sy, sz = ObjectGetPosition(ship)
    if nz < sz then
        -- 如果预测的坐标在船的下方，则不考虑
        return nil
    end
    local dx = nx - sx
    local dy = ny - sy
    local dz = nz - sz
    local squaredDistanceToShip = dx * dx + dy * dy + dz * dz
    if squaredDistanceToShip > squaredMaxShieldProtectionDistance then
        -- 如果预测的坐标距离船太远，则不考虑
        return nil
    end
    -- 检查区域附近是否有其他护盾
    local area = {
        X = nx,
        Y = ny,
        Z = nz,
        Radius = 0,
        DistType = "SPHERE"
    }
    local nearbyShields, nearbyShieldCount = ObjectFindObjects(ship, area, system.DuplicateShieldFilter)
    if nearbyShieldCount > 0 then
        -- 如果这个区域内已经有护盾了，则不考虑
        return nil
    end
    -- 在同一帧内新生成的护盾无法被 ObjectFindObjects 找到，所以需要单独检查
    -- 检查是否已经在这一帧内的附近位置生成过护盾
    local newShieldsAnchor = newShieldList.Anchor
    local newShieldNode = newShieldsAnchor.Next
    while newShieldNode ~= newShieldsAnchor do
        local nextNode = newShieldNode.Next
        local newShieldDistanceX = nx - newShieldNode.Nx
        local newShieldDistanceY = ny - newShieldNode.Ny
        local newShieldDistanceZ = nz - newShieldNode.Nz
        local squaredDistanceToNewShield =
            newShieldDistanceX * newShieldDistanceX +
            newShieldDistanceY * newShieldDistanceY +
            newShieldDistanceZ * newShieldDistanceZ
        if squaredDistanceToNewShield < squaredShieldDistanceGap then
            -- 这个位置已经生成过护盾了
            return nil
        end
        newShieldNode = nextNode
    end
    -- 确保护盾不会生成在单位内部（毕竟在那个位置的护盾也保护不了单位了）
    area.Radius = system.SHIELD_DISTANCE_GAP
    area.DistType = "EXTENTS_3D"
    local intersectedUnits, intersectedUnitCount = ObjectFindObjects(ship, area, system.OverlapWithAlliesFilter)
    if intersectedUnitCount ~= 0 then
        return nil
    end
    -- 如果这个区域内没有护盾，则可以在这里生成护盾
    -- 返回 area 里的 X, Y, Z 坐标
    -- 以及朝向（与敌方抛射体方向相反）

    -- 考虑护盾展开时还会向前移动一段距离
    local deployDistanceFactor = system.DEPLOY_DISTANCE / enemySpeed
    local result = {
        X = nx + enemyVx * deployDistanceFactor,
        Y = ny + enemyVy * deployDistanceFactor,
        Z = nz + enemyVz * deployDistanceFactor,
        -- 方向
        Vx = -enemyVx,
        Vy = -enemyVy,
        Vz = -enemyVz,
        -- 想要护盾抵达的位置
        Nx = nx,
        Ny = ny,
        Nz = nz
    }
    return result
end

function JapanAntiNavyShipShieldSystem.CreateNewShield(system, shieldSystemUnit, shieldPosition)
    -- 计算新护盾该有的朝向
    local vx, vy, vz = shieldPosition.Vx, shieldPosition.Vy, shieldPosition.Vz
    local length = sqrt(vx * vx + vy * vy + vz * vz)
    if length < system.VELOCITY_EPSILON then
        -- 速度太小，无法计算朝向
        return
    end
    vx, vy, vz = vx / length, vy / length, vz / length
    -- 准备新护盾所需的数据
    local playerScriptName = ObjectPlayerScriptName(shieldSystemUnit)
    local newShieldList = system.NewShieldListByPlayerName[playerScriptName]
    if newShieldList == nil then
        newShieldList = LinkedList()
        system.NewShieldListByPlayerName[playerScriptName] = newShieldList
    end
    local newShield = {
        FrameNumber = GetFrame(),
        HelperId = nil,
        ShieldId = nil,
        LinkedBeaconId = ObjectGetId(shieldSystemUnit),
        X = shieldPosition.X,
        Y = shieldPosition.Y,
        Z = shieldPosition.Z,
        -- 记录护盾想要到达的位置
        Nx = shieldPosition.Nx,
        Ny = shieldPosition.Ny,
        Nz = shieldPosition.Nz,
    }
    newShieldList:AppendNode(newShield)

    -- 准备创建护盾，护盾需要从一个坐标点发射出来
    -- 而这个坐标点并不一定是 shieldSystemUnit 的坐标点（或者说，并不是太刀的坐标点）
    -- 因此我们不能直接从 shieldSystemUnit 的坐标点发射护盾
    -- 而是需要创建一个“协助单位”，把协助单位放到正确的位置，然后让协助单位发射护盾
    ExecuteAction(
        "UNIT_SPAWN_NAMED_LOCATION_ORIENTATION",
        "",
        system.ShieldCreationHelperName,
        format("%s/%s", playerScriptName, ObjectTeamName(shieldSystemUnit)),
        shieldPosition,
        0
    )

    -- 协助单位创建的时候，会触发 OnCreated 事件，
    -- 并且会在 OnCreated 事件中设置 newShield.HelperUnit
    local creationHelper = newShield.HelperUnit
    if creationHelper ~= nil then
        -- 创建护盾
        ObjectCreateAndFireTempWeaponToTarget(
            creationHelper,
            system.CreateShieldProjectileWeapon,
            {
                X = shieldPosition.Nx + vx * system.EXTRA_DISTANCE,
                Y = shieldPosition.Ny + vy * system.EXTRA_DISTANCE,
                Z = shieldPosition.Nz + vz * system.EXTRA_DISTANCE,
            },
            nil
        )
    end

    -- 新护盾在创建的时候将会修改 newShield.ShieldUnit
    local shieldUnit = newShield.ShieldUnit
    if shieldUnit ~= nil then
        -- 让船只的充能子炮台重新考虑一下充能对象
        local ship = ObjectGetAttachees(shieldSystemUnit)[1]
        local table, count = ObjectGetContainedPassengers(ship)
        for i = 1, count, 1 do
            local turret = table[i]
            if ObjectTestTargetObjectWithFilter(ship, turret, system.RechargeShieldTurretFilter) then
                ExecuteAction("NAMED_ATTACK_NAMED", turret, shieldUnit)
            end
        end
    end
end

function JapanAntiNavyShipShieldSystem.ShieldUpdate(system, shieldUnit)
    -- 减少护盾血量
    if ObjectTestTargetObjectWithFilter(nil, shieldUnit, system.ShieldChargedFilter) then
        ExecuteAction("NAMED_DAMAGE", shieldUnit, system.SHIELD_LOW_DAMAGE_PER_FRAME)
    else
        ExecuteAction("NAMED_DAMAGE", shieldUnit, system.SHIELD_HIGH_DAMAGE_PER_FRAME)
    end

    -- 判断是否有抛射体将会撞到护盾
    JapanAntiNavyShipShieldSystem:UpdateShieldCollision(shieldUnit)
    -- 更新护盾颜色
    JapanAntiNavyShipShieldSystem:UpdateShieldColor(shieldUnit)
end

function JapanAntiNavyShipShieldSystem.UpdateShieldCollision(system, shieldUnit)
    local transform = ObjectGetTransform(shieldUnit)
    local x, y, z = transform[4], transform[8], transform[12]
    local enemyProjectiles, enemyCount = ObjectFindObjects(shieldUnit, {
        X = x, Y = y, Z = z,
        Radius = system.SHIELD_COLLISION_DETECTION_DISTANCE,
        DistType = "SPHERE"
    }, system.EnemyProjectileFilter)
    if enemyCount == 0 then
        return
    end
    -- 根据护盾的矩阵，得到逆矩阵
    -- 将敌方抛射体的坐标和速度通过逆矩阵转换到护盾的坐标系下
    -- 以便于计算抛射体是否会撞到护盾
    local worldToLocal = system:GetShieldInvertMatrix(shieldUnit)
    if worldToLocal == nil then
        -- 无法计算逆矩阵，这就坏了，这个护盾是否处于正常状态？还是杀了吧
        ExecuteAction("NAMED_KILL", shieldUnit)
        return
    end
    -- 逆矩阵
    local m11, m12, m13 = worldToLocal[1], worldToLocal[2], worldToLocal[3]
    local m21, m22, m23 = worldToLocal[4], worldToLocal[5], worldToLocal[6]
    local m31, m32, m33 = worldToLocal[7], worldToLocal[8], worldToLocal[9]
    -- 护盾大小
    local shieldSize = system.SHIELD_SIZE
    local shieldMinX, shieldMaxX = shieldSize.BACK, shieldSize.FRONT
    local shieldMinY, shieldMaxY = shieldSize.LEFT, shieldSize.RIGHT
    local shieldMinZ, shieldMaxZ = shieldSize.BOTTOM, shieldSize.TOP
    local epsilon = -system.VELOCITY_EPSILON
    for i = 1, enemyCount, 1 do
        -- 获取护盾当前的生命值
        local hp = ObjectGetCurrentHealth(shieldUnit)
        -- 获取敌方抛射体的坐标和速度，并转换到护盾的坐标系下
        local enemy = enemyProjectiles[i]
        local willCollide = false
        local ex, ey, ez = ObjectGetPosition(enemy)
        -- 确保依然存在
        -- 确保抛射体依然存在 - 假如它已经被引爆了，那么它的坐标就是 nil
        if ex ~= nil and hp ~= nil and hp > 1 then
            -- 通过抛射体在前一帧的坐标和当前坐标，计算抛射体的速度
            local vx, vy, vz = ObjectGetPreviousPosition(enemy)
            vx, vy, vz = ex - vx, ey - vy, ez - vz
            -- 把坐标和速度通过逆矩阵转换到护盾的坐标系下
            ex, ey, ez = ex - x, ey - y, ez - z
            ex, ey, ez =
                m11 * ex + m12 * ey + m13 * ez,
                m21 * ex + m22 * ey + m23 * ez,
                m31 * ex + m32 * ey + m33 * ez
            vx, vy, vz =
                m11 * vx + m12 * vy + m13 * vz,
                m21 * vx + m22 * vy + m23 * vz,
                m31 * vx + m32 * vy + m33 * vz
            -- 只处理抛射体的运动方向从正面突破护盾的情况
            if vx < -epsilon then
                -- 计算抛射体是否会撞到护盾
                -- 存在两种情况：
                -- 1. 抛射体现在已经在护盾内部了。
                --  这个是最好计算的，由于护盾在自身坐标系下，所以只需要判断抛射体的坐标大小即可
                -- 2. 抛射体在护盾外部
                --  这种情况下我们可以假设抛射体是一个点，然后判断抛射体的运动方向构成的直线是否会与护盾相交
                --  假如相交，我们再判断直线与护盾的交点是否在抛射体的位移之内
                -- x = ex + vx * t
                -- y = ey + vy * t
                -- z = ez + vz * t
                -- 设 x = ShieldMaxX，可以得到
                -- t = (ShieldMaxX - ex) / vx
                -- 假如 t 处于 [0, 1] 之间，则说明抛射体有可能会撞到护盾

                -- 检查抛射体是否已经在护盾内部了
                if shieldMinX <= ex and ex <= shieldMaxX and
                    shieldMinY <= ey and ey <= shieldMaxY and
                    shieldMinZ <= ez and ez <= shieldMaxZ then
                    -- 抛射体已经在护盾内部了
                    willCollide = true
                else
                    -- 把抛射体的运动路径视作直线
                    -- 假如它与护盾相交，那么它必然将穿过 x = shieldMinX 或者 x = shieldMaxX 的平面
                    local tByXMin = (shieldMinX - ex) / vx
                    local tByXMax = (shieldMaxX - ex) / vx
                    local yByT, zByT
                    -- 假如抛射体在过去 -1.5 帧之内、未来 1 帧之内的这个时间段就会穿过平面
                    if -1.5 <= tByXMin and tByXMin <= 1 then
                        -- 计算抛射体在 x = shieldMinX 平面上的坐标
                        yByT, zByT = ey + vy * tByXMin, ez + vz * tByXMin
                        -- 检查抛射体在那时是否会在护盾内部
                        if shieldMinY <= yByT and yByT <= shieldMaxY and
                            shieldMinZ <= zByT and zByT <= shieldMaxZ then
                            -- 抛射体已经（或者即将）与护盾产生碰撞
                            willCollide = true
                        end
                    end
                    if -1.5 <= tByXMax and tByXMax <= 1 then
                        -- 计算抛射体在 x = shieldMaxX 平面上的坐标
                        yByT, zByT = ey + vy * tByXMax, ez + vz * tByXMax
                        -- 检查抛射体在那时是否会在护盾内部
                        if shieldMinY <= yByT and yByT <= shieldMaxY
                            and shieldMinZ <= zByT and zByT <= shieldMaxZ then
                            willCollide = true
                        end
                    end
                end
                if willCollide then
                    -- 抛射体会撞到护盾，引爆抛射体
                    ObjectDetonateProjectile(enemy, shieldUnit)
                end
            end
        end
    end
end

function JapanAntiNavyShipShieldSystem.UpdateShieldColor(system, shieldUnit)
    -- 计算颜色
    local healthRatio = ObjectGetCurrentHealth(shieldUnit) / ObjectGetInitialHealth(shieldUnit)
    local color = system.SHIELD_COLOR
    if healthRatio < 0.3 then
        if 0.15 < healthRatio then
            color = system.SHIELD_DAMAGED_COLOR
        elseif 0 < healthRatio then
            color = system.SHIELD_CRITICAL_COLOR
        else
            color = system.SHIELD_DESTROYED_COLOR
        end
    end
    ExecuteAction("NAMED_CUSTOM_COLOR", shieldUnit, color[1] * 65536 + color[2] * 256 + color[3])
end

-- 计算护盾的逆矩阵
function JapanAntiNavyShipShieldSystem.GetShieldInvertMatrix(system, shieldUnit)
    local transform = ObjectGetTransform(shieldUnit)
    -- 我们的矩阵是
    -- { [ 1], [ 2], [ 3], [ 4], 
    --   [ 5], [ 6], [ 7], [ 8],
    --   [ 9], [10], [11], [12], 
    --   [13], [14], [15], [16] }
    -- 实际上，右侧是位移，底侧是 0 0 0 1
    -- 因此实际上我们只需要计算左上角的 3x3 矩阵
    
    -- 创建转置矩阵
    local t11, t12, t13 = transform[1], transform[5], transform[9]
    local t21, t22, t23 = transform[2], transform[6], transform[10]
    local t31, t32, t33 = transform[3], transform[7], transform[11]

    local det11Minor = t22 * t33 - t23 * t32
    local det12Minor = t21 * t33 - t23 * t31
    local det13Minor = t21 * t32 - t22 * t31
    local det21Minor = t12 * t33 - t13 * t32
    local det22Minor = t11 * t33 - t13 * t31
    local det23Minor = t11 * t32 - t12 * t31
    local det31Minor = t12 * t23 - t13 * t22
    local det32Minor = t11 * t23 - t13 * t21
    local det33Minor = t11 * t22 - t12 * t21

    local determinant = t11 * det11Minor - t12 * det12Minor + t13 * det13Minor
    if determinant == 0 then
        return nil
    end

    local invDet = 1 / determinant
    return {
        invDet * det11Minor, -invDet * det12Minor, invDet * det13Minor,
        -invDet * det21Minor, invDet * det22Minor, -invDet * det23Minor,
        invDet * det31Minor, -invDet * det32Minor, invDet * det33Minor,
    }
end
```
