# -*- coding: utf-8 -*-
"""Validate a RefPack stream against the original data, reporting the first divergence."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))


def validate(comp: bytes, raw: bytes, limit=40):
    data = comp
    dsize = (data[2] << 16) | (data[3] << 8) | data[4]
    pos = 5
    out = bytearray()
    steps = 0
    while True:
        b1 = data[pos]; pos += 1
        kind = None
        if (b1 & 0x80) == 0:
            b2 = data[pos]; pos += 1
            lit = b1 & 3
            out += data[pos:pos + lit]; pos += lit
            rlen = ((b1 & 0x1C) >> 2) + 3
            dist = ((b1 & 0x60) << 3) + b2 + 1
            kind = 'M2'
        elif (b1 & 0x40) == 0:
            b2, b3 = data[pos], data[pos + 1]; pos += 2
            lit = (b2 & 0xC0) >> 6
            out += data[pos:pos + lit]; pos += lit
            rlen = (b1 & 0x3F) + 4
            dist = ((b2 & 0x3F) << 8) + b3 + 1
            kind = 'M3'
        elif (b1 & 0x20) == 0:
            b2, b3, b4 = data[pos], data[pos + 1], data[pos + 2]; pos += 3
            lit = b1 & 3
            out += data[pos:pos + lit]; pos += lit
            rlen = ((b1 & 0x0C) << 6) + b4 + 5
            dist = ((b1 & 0x10) << 12) + (b2 << 8) + b3 + 1
            kind = 'M4'
        else:
            if b1 < 0xFC:
                lit = ((b1 & 0x1F) + 1) << 2
                out += data[pos:pos + lit]; pos += lit
                steps += 1
                continue
            lit = b1 & 0x03
            out += data[pos:pos + lit]; pos += lit
            print('STOP outlen=%d expected=%d  ok=%s' % (len(out), dsize, bytes(out) == raw[:len(out)]))
            return
        if dist > len(out):
            print('BAD REF at step %d: kind=%s outlen=%d dist=%d rlen=%d' % (steps, kind, len(out), dist, rlen))
            print('  out tail:', bytes(out[-24:]).hex(' '))
            print('  raw at outlen:', raw[max(0, len(out) - 24):len(out) + 8].hex(' '))
            return
        for _ in range(rlen):
            out.append(out[-dist])
        if bytes(out) != raw[:len(out)]:
            print('DIVERGE at step %d kind=%s outlen=%d' % (steps, kind, len(out)))
            print('  got:', bytes(out[-24:]).hex(' '))
            print('  exp:', raw[len(out) - 24:len(out)].hex(' '))
            return
        steps += 1
        if steps > 2000000:
            print('too many steps'); return


if __name__ == '__main__':
    from refpack_enc import refpack_compress
    raw = open(sys.argv[1], 'rb').read()
    n = int(sys.argv[2]) if len(sys.argv) > 2 else len(raw)
    raw = raw[:n]
    comp = refpack_compress(raw)
    print('compressed %d -> %d' % (len(raw), len(comp)))
    validate(comp, raw)
