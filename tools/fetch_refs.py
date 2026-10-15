# -*- coding: utf-8 -*-
import urllib.request, os, time

FILES = """Asset.cs AssetList.cs AssetNameCollection.cs AssetProperty.cs AssetPropertyCollection.cs
AssetPropertyKey.cs AssetPropertyType.cs BlendDescription.cs BlendDirection.cs BlendFlags.cs
BlendTileData.cs BlendTileTexture.cs BlendTileTextureIndex.cs BuildList.cs BuildLists.cs
CameraAnimation.cs CameraAnimationFrame.cs CameraAnimationList.cs CastlePerimeter.cs
CastlePerimeterPoint.cs CastleTemplate.cs CastleTemplates.cs CliffTextureMapping.cs
EnvironmentData.cs FogSettings.cs GlobalLight.cs GlobalLighting.cs GlobalLightingConfiguration.cs
GlobalVersion.cs GlobalWaterSettings.cs HeightMapData.cs LibraryMapLists.cs LibraryMaps.cs
MPPositionInfo.cs MPPositionList.cs MapColorArgb.cs MapFile.cs MapObject.cs MapParseContext.cs
MissionHotSpot.cs MissionHotSpots.cs MissionObjective.cs MissionObjectives.cs NamedCamera.cs
NamedCameras.cs ObjectsList.cs Player.cs PlayerScriptsList.cs PolygonTriggers.cs PostEffect.cs
PostEffectsChunk.cs RiverArea.cs RiverAreas.cs RoadType.cs SkyboxSettings.cs StandingWaterArea.cs
StandingWaterAreas.cs StandingWaveArea.cs StandingWaveAreas.cs Team.cs Teams.cs TriggerArea.cs
TriggerAreas.cs UserMapCache.cs WaypointPath.cs WaypointsList.cs WorldInfo.cs""".split()

BASE = 'https://raw.githubusercontent.com/OpenSAGE/OpenSAGE/master/src/OpenSage.Game/Data/Map/'
outdir = 'D:/workspace/ra3maps/refs'
os.makedirs(outdir, exist_ok=True)

# also grab binary reader/writer extensions
EXTRA = [
    ('https://raw.githubusercontent.com/OpenSAGE/OpenSAGE/master/src/OpenSage.Game/Data/Utilities/Extensions/BinaryReaderExtensions.cs', 'BinaryReaderExtensions.cs'),
    ('https://raw.githubusercontent.com/OpenSAGE/OpenSAGE/master/src/OpenSage.Game/Data/Utilities/Extensions/BinaryWriterExtensions.cs', 'BinaryWriterExtensions.cs'),
]

ok = fail = 0
todo = [(BASE + f, f) for f in FILES] + EXTRA
for url, fn in todo:
    dst = os.path.join(outdir, fn)
    if os.path.exists(dst) and os.path.getsize(dst) > 0:
        ok += 1
        continue
    for attempt in range(4):
        try:
            r = urllib.request.Request(url, headers={'User-Agent': 'dsh'})
            data = urllib.request.urlopen(r, timeout=30).read()
            open(dst, 'wb').write(data)
            ok += 1
            break
        except Exception as e:
            if attempt == 3:
                print('FAIL', fn, e)
                fail += 1
            else:
                time.sleep(2)
print('downloaded', ok, 'failed', fail)
