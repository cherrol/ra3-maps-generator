---
title: "RegisteredObjectSetId 清单"
catalog_path: "Lua API "
slug: nx856npyys4vtp53
source: https://www.yuque.com/muzeqaq/ra3mapwiki/nx856npyys4vtp53
fetched: 2026-10-15
content_updated_at: "2024-11-28T22:21:16.000Z"
type: Doc
format: lake
---

# RegisteredObjectSetId 清单

> 原文：https://www.yuque.com/muzeqaq/ra3mapwiki/nx856npyys4vtp53
> 目录位置：Lua API 

```xml
<xs:simpleType name="RegisteredObjectSetId">
    <xs:restriction base="xs:string">

      <xs:enumeration value="PLAYER_EVERYTHING"/>

      <!-- Global categories -->
      <xs:enumeration value="GARRISONS"/>
      <xs:enumeration value="NEUTRAL_TECH"/>
      <xs:enumeration value="TIBERIUM_FIELDS" />
      <xs:enumeration value="ORE_MINES" />
      <xs:enumeration value="SIGMA_FIELDS" />
      <xs:enumeration value="GLOBAL_HARVESTERS" />
      <xs:enumeration value="GLOBAL_TIBERIUM_REFINERIES" />
      <xs:enumeration value="GLOBAL_ORE_REFINERIES" />
      <xs:enumeration value="GLOBAL_SIGMA_REFINERIES" />

      <!-- High level categories -->
      <xs:enumeration value="STRUCTURES" />
      <xs:enumeration value="FACTION_STRUCTURES" />
      <xs:enumeration value="PLAYER_NEUTRAL_TECH"/>
      <xs:enumeration value="UNITS" />
      <xs:enumeration value="BEACONS" />
      <xs:enumeration value="UNITS_AND_STRUCTURES" />
      <xs:enumeration value="BASE_DEFENSES_AND_GARRISONS"/>

      <!-- Unit sub-categories -->
      <xs:enumeration value="MOBILE_GROUND_UNITS" />

      <xs:enumeration value="MCVS" />
      <xs:enumeration value="DOZERS" />
      <xs:enumeration value="HARVESTERS" />
      <xs:enumeration value="SURVEYORS" />
      <xs:enumeration value="OUTPOSTS" />
      <xs:enumeration value="INFANTRY" />
      <xs:enumeration value="VEHICLES" />
      <xs:enumeration value="SHIPS" />
      <xs:enumeration value="AIRCRAFT" />
      <xs:enumeration value="UNPACKS_INTO_BUILDING_UNITS" />

      <xs:enumeration value="HORDES" />
      <xs:enumeration value="HORDE_MEMBERS" />

      <xs:enumeration value="ENGINEERS" />
      <xs:enumeration value="ENEMY_ENGINEERS" />

      <!-- Structure sub-categories -->
      <xs:enumeration value="VICTORY_CONDITION_STRUCTURES" />

      <xs:enumeration value="CONYARDS" />
      <xs:enumeration value="FACTORIES" />
      <xs:enumeration value="UNIT_FACTORIES"/>

      <xs:enumeration value="RADARS" />
      <xs:enumeration value="TECH_CENTERS" />

      <xs:enumeration value="POWER_PLANTS" />
      <xs:enumeration value="REFINERIES" />
      <xs:enumeration value="REFINERIES_AND_REFINERY_EGGS" />
      <xs:enumeration value="TIBERIUM_REFINERIES" />
      <xs:enumeration value="ORE_REFINERIES" />
      <xs:enumeration value="SIGMA_REFINERIES" />
      <xs:enumeration value="SILOS" />
      <xs:enumeration value="FARMS" />

      <xs:enumeration value="BASE_DEFENSES" />
      <xs:enumeration value="ENEMY_BASE_DEFENSES" />

      <!-- Player power updatable category -->
      <xs:enumeration value="AI_PLAYER_POWER_UPDATABLE" />

      <xs:enumeration value="SKIRMISH_AI_DEFENSE_TARGETS" />
      <xs:enumeration value="SKIRMISH_AI_THREATS_TO_DODGE" />
      <xs:enumeration value="SKIRMISH_AI_NORMAL_COMBAT_UNITS" />

    <!-- RA3 NanoSwarmHive, this should be removed for non-RA3 projects -->
    <xs:enumeration value="ENCLOSURES" />
    </xs:restriction>
  </xs:simpleType>
```
