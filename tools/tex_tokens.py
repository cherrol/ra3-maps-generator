# -*- coding: utf-8 -*-
"""Derive exact (name, tm_token, nrm_token) triplets from working maps."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *

M = os.path.join(os.environ['APPDATA'], 'Red Alert 3', 'Maps')
WANT = ['Grass_Yucatan07', 'Grass_Yucatan02', 'Grass_Yucatan09', 'Rock_Yucatan01', 'Grass_Yucatan06',
        'Grass_Yucatan05', 'Grass_Yucatan03', 'Grass_Yucatan04', 'RA3_Elevation1', 'Grass_Yucatan08',
        'Transition_Yucatan18', 'RA3_Elevation0', 'Grass_Yucatan01', 'Transition_Yucatan14',
        'Gravel_Yucatan01', 'Dirt_Yucatan06', 'Transition_Yucatan13', 'Transition_Yucatan16',
        'Transition_Yucatan20', 'Transition_Yucatan15', 'Transition_Yucatan04', 'Dirt_Yucatan03',
        'Transition_Yucatan03', 'Dirt_Yucatan05', 'Transition_Yucatan08', 'Dirt_Yucatan04',
        'Rock_Yucatan02', 'RA3_ShallowSeaFloor', 'Dirt_Yucatan02', 'Dirt_Yucatan01',
        'Reef_Yucatan02', 'Reef_Havana02', 'Reef_Yucatan01', 'Transition_Yucatan02',
        'Rock_Yucatan04', 'Rock_Yucatan03', 'Transition_Yucatan06', 'Transition_Yucatan19',
        'Transition_Yucatan21', 'SteelDeck01', 'Sand_Hawaii02', 'BB_Gravel01']


def pairs(folder):
    d = os.path.join(M, folder)
    mp = [f for f in os.listdir(d) if f.lower().endswith('.map')][0]
    m = MapFile.load(os.path.join(d, mp))
    hm = parse_heightmap(m.get('HeightMapData'))
    bt = parse_blendtiles(m.get('BlendTileData'), hm['width'], hm['height'], m.table)
    r = R(m.get('WorldInfo').payload)
    props = read_props(r, m.table)
    s = [v for k, t, v in props if k == 'terrainTextureStrings'][0].rstrip(';').split(';')
    toks = [(s[i], s[i + 1]) for i in range(0, len(s) - 1, 2)]
    names = [t['name'] for t in bt['textures']]
    return names, toks


table = {}
for folder in ('{8p_1.5}_death_aquatic', '【44gg】海陆中枢(爽+)_mix'):
    names, toks = pairs(folder)
    print(folder, 'names', len(names), 'tokens', len(toks))
    for i, n in enumerate(names):
        if i < len(toks):
            table[n] = toks[i]

lines = []
for n in WANT:
    a, b = table.get(n, ('?', '?'))
    if a == '?':
        print('MISSING', n)
    lines.append("    ('%s', '%s', '%s')," % (n, a, b))
open(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'analysis', 'tex_tokens.txt'),
     'w', encoding='utf-8').write('\n'.join(lines))
print('\n'.join(lines))
