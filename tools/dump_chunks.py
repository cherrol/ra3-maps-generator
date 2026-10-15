# -*- coding: utf-8 -*-
"""Dump chunk inventory of a RA3 .map (decompressed CkMp payload)."""
import sys, struct

def read_string_table(data, pos):
    n, = struct.unpack_from('<I', data, pos); pos += 4
    table = {}
    for _ in range(n):
        # 7-bit varint length
        shift = 0; ln = 0
        while True:
            b = data[pos]; pos += 1
            ln |= (b & 0x7F) << shift
            if not (b & 0x80): break
            shift += 7
        s = data[pos:pos+ln].decode('ascii'); pos += ln
        idx, = struct.unpack_from('<I', data, pos); pos += 4
        table[idx] = s
    return table, pos

def main(path):
    data = open(path, 'rb').read()
    assert data[:4] == b'CkMp'
    table, pos = read_string_table(data, 4)
    print(f'strings: {len(table)}')
    ci = 0
    while pos < len(data):
        name_idx, ver, size = struct.unpack_from('<IHI', data, pos)
        name = table.get(name_idx, f'#{name_idx}')
        print(f'{ci:3d} off={pos:9d} ver={ver:3d} size={size:9d} {name}')
        pos += 10 + size
        ci += 1
    print(f'end pos={pos} len={len(data)}')

if __name__ == '__main__':
    main(sys.argv[1])
