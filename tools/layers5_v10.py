# -*- coding: utf-8 -*-
"""五层预览 v4：合并力度更大 + 阈值按地形自适应（分位数）+ 第五层高山 480~1000（占比刻意压小）。
   保留 v1/v2/v3 的图。不改地图数据。
"""
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import terrain_gen as bt          # noqa: E402

LEVELS = [210.0, 280.0, 350.0, 420.0, 480.0]        # 480 = 高山（不可通行）
COL = {210: (150, 200, 120), 280: (110, 175, 95), 350: (170, 165, 80),
       420: (190, 130, 70), 480: (235, 235, 240)}
# 自适应分位（在"陆地且非高山"范围内切四刀）；高山取最顶上的很小一块
Q = [0.34, 0.62, 0.82]          # 210 | 280 | 350 | 420
Q_PEAK = 0.965                  # 顶部 3.5% 作为高山层
R = 4                           # 合并半径（比 v3 的 3 更大）
PASSES = 3
KEEP_PER_LAYER = 22             # 每层保留的块数（越小合并越狠）


def box_sum(mask, r):
    c = np.cumsum(np.cumsum(mask.astype(np.int32), axis=0), axis=1)
    c = np.pad(c, ((1, 0), (1, 0)))
    H, W = mask.shape
    y0 = np.clip(np.arange(H) - r, 0, H); y1 = np.clip(np.arange(H) + r + 1, 0, H)
    x0 = np.clip(np.arange(W) - r, 0, W); x1 = np.clip(np.arange(W) + r + 1, 0, W)
    return (c[np.ix_(y1, x1)] - c[np.ix_(y0, x1)] - c[np.ix_(y1, x0)] + c[np.ix_(y0, x0)])


def majority(lvl, r, passes, fixed=None):
    vals = list(LEVELS) + [0.0]
    orig = lvl.copy()
    for _ in range(passes):
        counts = np.stack([box_sum(lvl == v, r) for v in vals])
        best = np.argmax(counts, axis=0)
        cur = np.zeros(lvl.shape, np.int32)
        for i, v in enumerate(vals):
            cur = np.where(lvl == v, i, cur)
        bestc = np.take_along_axis(counts, best[None], axis=0)[0]
        curc = np.take_along_axis(counts, cur[None], axis=0)[0]
        best = np.where(bestc == curc, cur, best)
        lvl = np.array(vals)[best]
        if fixed is not None:
            lvl = np.where(fixed, orig, lvl)
    return lvl


def keep_top_components(lvl, keep):
    out = lvl.copy()
    total = 0
    for lv in LEVELS:
        m = (out == lv)
        if not m.any():
            continue
        drop = m & (~bt.keep_largest_components(m, keep))
        if not drop.any():
            continue
        counts = {l2: box_sum((out == l2) & ~drop, 3).astype(np.float64) for l2 in LEVELS if l2 != lv}
        stack = np.stack([counts[l2] for l2 in counts])
        pick = np.argmax(stack, axis=0)
        for i, l2 in enumerate(counts):
            out[drop & (pick == i)] = l2
        total += int(drop.sum())
    return out, total


def render(lvl, path, title, cuts):
    img = np.zeros(lvl.shape + (3,), dtype=np.uint8)
    for lv in LEVELS:
        img[lvl == lv] = COL[lv]
    img[lvl == 0] = (36, 88, 156)
    gy, gx = np.gradient(lvl)
    shade = np.clip(1.0 - 0.04 * (gx + gy) / max(1.0, np.abs(gx).max() + 1e-6) * 20.0, 0.84, 1.14)
    img = np.clip(img.astype(np.float64) * shade[..., None], 0, 255).astype(np.uint8)
    im = Image.fromarray(np.flipud(img)).resize((900, 900), Image.NEAREST)
    d = ImageDraw.Draw(im)
    for i, lv in enumerate(LEVELS):
        pct = 100.0 * (lvl == lv).sum() / lvl.size
        x = 16 + (i % 2) * 430
        y = 16 + (i // 2) * 30
        d.rectangle([x, y, x + 24, y + 22], fill=COL[lv], outline=(0, 0, 0))
        d.text((x + 32, y + 3), '%d  %.1f%%' % (lv, pct), fill=(255, 255, 255))
    sx = 16 + (len(LEVELS) % 2) * 430
    sy = 16 + (len(LEVELS) // 2) * 30
    d.rectangle([sx, sy, sx + 24, sy + 22], fill=(36, 88, 156), outline=(0, 0, 0))
    d.text((sx + 32, sy + 3), 'sea  %.1f%%' % (100.0 * (lvl == 0).sum() / lvl.size), fill=(255, 255, 255))
    d.text((16, 852), title, fill=(255, 255, 255))
    d.text((16, 872), 'adaptive cuts(m): %s' % ['%.0f' % c for c in cuts], fill=(255, 255, 255))
    im.save(path)


def main():
    base = os.path.dirname(os.path.abspath(bt.cfg.__file__))
    dem = np.load(os.path.join(base, 'dem.npz'))['dem'].astype(np.float64)
    land = dem > 0
    d = dem[land]

    # ---- 阈值按地形自适应：先切出高山（顶部 Q_PEAK），再在其余陆地上按分位切四层
    peak_cut = float(np.quantile(d, Q_PEAK))
    rest = d[d < peak_cut]
    cuts = [float(np.quantile(rest, q)) for q in Q]
    print('自适应阈值(m)：%s   高山门槛 %.0f （顶部 %.1f%%）' % (
        ['%.0f' % c for c in cuts], peak_cut, (1 - Q_PEAK) * 100))

    lvl = np.full(dem.shape, LEVELS[0], dtype=np.float64)
    lvl = np.where(dem >= cuts[0], LEVELS[1], lvl)
    lvl = np.where(dem >= cuts[1], LEVELS[2], lvl)
    lvl = np.where(dem >= cuts[2], LEVELS[3], lvl)
    lvl = np.where(dem >= peak_cut, LEVELS[4], lvl)
    lvl = np.where(dem <= 0, 0.0, lvl)

    # （不做" 界以南填陆地\的裁切：那会把南边陆地也切掉；改为只留贴岸窄带，见下）

    # 海：只固定最大的一片；高山层也要保护（不能被投票吃掉）
    sea = (lvl == 0.0)
    small_sea = sea & (~bt.keep_largest_components(sea, 1))
    fixed = (sea & (~small_sea)) | (lvl == LEVELS[4])

    lvl4 = majority(lvl, R, PASSES, fixed=fixed)
    lvl4, ch = keep_top_components(lvl4, KEEP_PER_LAYER)
    lvl4 = majority(lvl4, 2, 1, fixed=fixed)

    print('v4 占比: ' + '  '.join('%d=%.1f%%' % (lv, 100.0 * (lvl4 == lv).sum() / lvl4.size)
                                  for lv in LEVELS) + '  sea=%.1f%%' % (100.0 * (lvl4 == 0).sum() / lvl4.size))
    print('v4 合并掉的零散陆地: %d 格 ; 参投小水域: %d 格 ; 高山 %d 格' % (
        ch, int(small_sea.sum()), int((lvl4 == LEVELS[4]).sum())))
    # ---- 只保留离陆地 25 格以内的海（自然形成贴岸窄带，其余海面并成陆地）
    sea = (lvl4 == 0.0)
    if sea.any():
        d_land = bt.distance_transform(~sea)
        # 新规则：范围内的海域全部保留（不再把远海并成陆地）
        print('海域保留：范围内海面 %d 格（%.1f%%）' % (int(sea.sum()), 100.0*sea.mean()))
    lvl4, _ch2 = keep_top_components(lvl4, KEEP_PER_LAYER)
    lvl4 = majority(lvl4, 2, 1, fixed=(lvl4 == LEVELS[4]))

    render(lvl4, os.path.join(base, 'layers5_preview_v10.png'),
           'v5  r=%d passes=%d keep<=%d/layer  (5 layers, 480 = impassable, 南边海域已北缩)' % (R, PASSES, KEEP_PER_LAYER),
           cuts + [peak_cut])
    print('已保存 layers5_preview_v10.png（v1~v4 保留）')


if __name__ == '__main__':
    main()
