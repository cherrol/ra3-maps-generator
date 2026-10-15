# -*- coding: utf-8 -*-
"""RefPack encoder with back-references (EA RefPack / DBPF LZ77).

Opcode layout verified against OpenSAGE RefPackStream (decoder):
  2-byte: 0LLLLMMM DDDDDDDD            lit 0-3, len 3-10,   dist 1-1024
  3-byte: 10MMMMMM LDDDDDDD DDDDDDDD   lit 0-3, len 4-67,   dist 1-16384
  4-byte: 1100LLMM LLDDDDDD DDDDDDDD DDDDDDDD   lit 0-3, len 5-1028, dist 1-131072
  1-byte: 111nnnnn (n literal dwords)  len 4-112
  stop:   111111LL (+ 0-3 literals)
"""
import struct

MAX_WINDOW = 131072
MIN_MATCH = 3
MAX_MATCH = 1028


def refpack_compress(data: bytes, max_chain=24) -> bytes:
    n = len(data)
    out = bytearray([0x10, 0xFB, (n >> 16) & 0xFF, (n >> 8) & 0xFF, n & 0xFF])
    if n >= (1 << 24):
        raise ValueError('decompressed size too large for the 3-byte header variant')

    # hash chains: hash of 3 bytes -> list of positions (bounded by max_chain)
    head = {}
    prev = [-1] * n

    def find_match(pos):
        if pos + MIN_MATCH > n:
            return 0, 0
        key = data[pos:pos + 3]
        cand = head.get(key)
        best_len, best_dist = 0, 0
        chain = 0
        while cand is not None and chain < max_chain:
            dist = pos - cand
            if dist > MAX_WINDOW:
                break
            # compare
            maxlen = min(MAX_MATCH, n - pos)
            l = 0
            while l < maxlen and data[cand + l] == data[pos + l]:
                l += 1
            if l > best_len:
                best_len, best_dist = l, dist
                if l >= 512:
                    break
            cand = prev[cand] if 0 <= cand < n and prev[cand] >= 0 and pos - prev[cand] <= MAX_WINDOW else None
            chain += 1
        return best_len, best_dist

    def insert(pos):
        if pos + MIN_MATCH <= n:
            key = data[pos:pos + 3]
            prev[pos] = head.get(key, -1)
            head[key] = pos

    def emit_literals(start, count):
        """Emit `count` literals (>=0) using 1-byte literal commands."""
        p = start
        left = count
        while left >= 4:
            chunk = min((left // 4) * 4, 112)
            out.append(0xE0 | ((chunk // 4) - 1))
            out.extend(data[p:p + chunk])
            p += chunk
            left -= chunk
        return left          # 0..3 remaining, carried by the next command

    pos = 0
    lit_start = 0          # start of pending literals
    pending = 0            # pending literal count (0..3 after emit_literals)
    while pos < n:
        mlen, dist = find_match(pos)
        use = 0
        if mlen >= MIN_MATCH:
            if dist <= 1024 and mlen <= 10:
                use = 2
            elif dist <= 16384 and 4 <= mlen <= 67:
                use = 3
            elif mlen >= 5:
                use = 4
        if use:
            lit = pending
            lit_bytes = data[lit_start:lit_start + lit]
            if use == 2:
                b1 = (lit & 3) | ((mlen - 3) << 2) | (((dist - 1) >> 8) << 5)
                out.append(b1)
                out.append((dist - 1) & 0xFF)
                out.extend(lit_bytes)
            elif use == 3:
                d = dist - 1
                out.append(0x80 | (mlen - 4))
                out.append(((lit & 3) << 6) | ((d >> 8) & 0x3F))
                out.append(d & 0xFF)
                out.extend(lit_bytes)
            else:
                d = dist - 1
                if mlen > MAX_MATCH:
                    mlen = MAX_MATCH
                out.append(0xC0 | (lit & 3) | (((mlen - 5) >> 8) << 2) | (((d >> 16) & 1) << 4))
                out.append((d >> 8) & 0xFF)
                out.append(d & 0xFF)
                out.append((mlen - 5) & 0xFF)
                out.extend(lit_bytes)
            # register positions covered by the match
            for k in range(pos, min(pos + mlen, n)):
                insert(k)
            pos += mlen
            pending = 0
            lit_start = pos
        else:
            insert(pos)
            pos += 1
            pending += 1
            if pending >= 4:
                pending = emit_literals(lit_start, pending)
                lit_start = pos - pending
    if pending:
        out.append(0xFC | pending)
        out.extend(data[lit_start:lit_start + pending])
    else:
        out.append(0xFC)
    return bytes(out)


if __name__ == '__main__':
    import sys, os
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from refpack import refpack_decompress
    p = sys.argv[1]
    raw = open(p, 'rb').read()
    comp = refpack_compress(raw)
    back = refpack_decompress(comp)
    print(f'{p}: {len(raw)} -> {len(comp)} ({100.0*len(comp)/len(raw):.1f}%)  roundtrip={back == raw}')
