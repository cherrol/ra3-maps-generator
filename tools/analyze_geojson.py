# -*- coding: utf-8 -*-
"""Analyze Guangxi GeoJSON: bbox, polygon counts, point counts."""
import json, sys

def main(path):
    gj = json.load(open(path, encoding='utf-8'))
    feats = gj.get('features', [])
    print('features:', len(feats))
    polys = []
    for f in feats:
        g = f['geometry']
        if g['type'] == 'Polygon':
            polys.append(g['coordinates'])
        elif g['type'] == 'MultiPolygon':
            polys.extend(g['coordinates'])
    print('polygon parts:', len(polys))
    allpts = []
    for i, p in enumerate(polys):
        ring = p[0]
        xs = [c[0] for c in ring]; ys = [c[1] for c in ring]
        print(f'part {i}: pts={len(ring)} bbox=({min(xs):.3f},{min(ys):.3f})-({max(xs):.3f},{max(ys):.3f})')
        allpts.extend(ring)
    xs = [c[0] for c in allpts]; ys = [c[1] for c in allpts]
    print(f'TOTAL bbox: lon {min(xs):.4f}..{max(xs):.4f}  lat {min(ys):.4f}..{max(ys):.4f}')

if __name__ == '__main__':
    main(sys.argv[1])
