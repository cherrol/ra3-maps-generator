# -*- coding: utf-8 -*-
"""Rasterize Guangxi GeoJSON to a land mask + preview PNG."""
import json, sys
from PIL import Image, ImageDraw

def main(src, outpng, W=900, H=700, pad_frac=0.03):
    gj = json.load(open(src, encoding='utf-8'))
    polys = []
    for f in gj['features']:
        g = f['geometry']
        if g['type'] == 'Polygon':
            polys.append(g['coordinates'][0])
        elif g['type'] == 'MultiPolygon':
            for p in g['coordinates']:
                polys.append(p[0])
    xs = [c[0] for p in polys for c in p]
    ys = [c[1] for p in polys for c in p]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    dx, dy = x1 - x0, y1 - y0
    x0 -= dx * pad_frac; x1 += dx * pad_frac
    y0 -= dy * pad_frac; y1 += dy * pad_frac
    def tr(x, y):
        px = (x - x0) / (x1 - x0) * (W - 1)
        py = (1 - (y - y0) / (y1 - y0)) * (H - 1)
        return (px, py)
    img = Image.new('RGB', (W, H), (30, 60, 120))
    d = ImageDraw.Draw(img)
    for p in polys:
        d.polygon([tr(*c) for c in p], fill=(90, 160, 90))
    # city borders overlay (draw each feature outline)
    for f in gj['features']:
        g = f['geometry']
        rings = []
        if g['type'] == 'Polygon':
            rings = [g['coordinates'][0]]
        else:
            rings = [pp[0] for pp in g['coordinates']]
        for r in rings:
            d.line([tr(*c) for c in r], fill=(200, 200, 120), width=1)
    img.save(outpng)
    print('saved', outpng, f'bbox lon {x0:.3f}..{x1:.3f} lat {y0:.3f}..{y1:.3f}')

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2])
