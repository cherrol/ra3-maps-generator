# -*- coding: utf-8 -*-
"""RefPack decompressor (port of OpenSAGE RefPackStream) + literals-only compressor."""
import struct

def refpack_decompress(data: bytes) -> bytes:
    # Some community maps (e.g. the 44gg series) write the large-file variant as
    # 0x80 instead of 0x90 (base marker 0x10 replaced by the large flag).
    if data[0] == 0x80 or data[0] == 0x81:
        data = bytes([data[0] | 0x10]) + data[1:]
    assert (data[0] & 0b00111110) == 0b00010000, 'bad refpack magic b1: %02x' % data[0]
    assert data[1] == 0xFB, 'bad refpack magic b2'
    large = bool(data[0] & 0x80)
    has_comp = bool(data[0] & 0x01)
    pos = 2
    def read_size(p):
        n = 4 if large else 3
        v = 0
        for i in range(n):
            v = (v << 8) | data[p + i]
        return v, p + n
    if has_comp:
        _, pos = read_size(pos)
    dsize, pos = read_size(pos)
    out = bytearray()
    while True:
        b1 = data[pos]; pos += 1
        if (b1 & 0x80) == 0:  # 2-byte
            b2 = data[pos]; pos += 1
            lit = b1 & 0x03
            out += data[pos:pos+lit]; pos += lit
            rlen = ((b1 & 0x1C) >> 2) + 3
            dist = ((b1 & 0x60) << 3) + b2 + 1
            for _ in range(rlen):
                out.append(out[-dist])
        elif (b1 & 0x40) == 0:  # 3-byte
            b2, b3 = data[pos], data[pos+1]; pos += 2
            lit = (b2 & 0xC0) >> 6
            out += data[pos:pos+lit]; pos += lit
            rlen = (b1 & 0x3F) + 4
            dist = ((b2 & 0x3F) << 8) + b3 + 1
            for _ in range(rlen):
                out.append(out[-dist])
        elif (b1 & 0x20) == 0:  # 4-byte
            b2, b3, b4 = data[pos], data[pos+1], data[pos+2]; pos += 3
            lit = b1 & 0x03
            out += data[pos:pos+lit]; pos += lit
            rlen = ((b1 & 0x0C) << 6) + b4 + 5
            dist = ((b1 & 0x10) << 12) + (b2 << 8) + b3 + 1
            for _ in range(rlen):
                out.append(out[-dist])
        else:  # 1-byte
            if b1 < 0xFC:
                lit = ((b1 & 0x1F) + 1) << 2
                out += data[pos:pos+lit]; pos += lit
            else:
                lit = b1 & 0x03
                out += data[pos:pos+lit]; pos += lit
                break
    assert len(out) == dsize, f'size mismatch {len(out)} != {dsize}'
    return bytes(out)

def refpack_compress_literals(data: bytes) -> bytes:
    """Valid RefPack stream using only 1-byte literal commands + stop. No back-references."""
    dsize = len(data)
    if dsize >= (1 << 24):
        raise ValueError('too big for 3-byte size')
    out = bytearray([0x10, 0xFB])
    out += bytes([(dsize >> 16) & 0xFF, (dsize >> 8) & 0xFF, dsize & 0xFF])
    pos = 0
    n = dsize
    while n - pos >= 4:
        chunk = min(((n - pos) // 4) * 4, 112)
        out.append(0xE0 | ((chunk // 4) - 1))
        out += data[pos:pos+chunk]
        pos += chunk
    rem = n - pos  # 0..3
    out.append(0xFC | rem)
    out += data[pos:pos+rem]
    return bytes(out)

def load_map(path: str) -> bytes:
    """Load a .map file, return decompressed plaintext (starts with CkMp)."""
    data = open(path, 'rb').read()
    magic = data[:4]
    if magic == b'CkMp':
        return data
    if magic == b'EAR\x00':
        return refpack_decompress(data[8:])
    raise ValueError(f'unknown magic {magic!r}')

def save_map(plaintext: bytes, compress=True) -> bytes:
    """Wrap plaintext (CkMp...) into EAR-compressed .map bytes."""
    assert plaintext[:4] == b'CkMp'
    if compress:
        from refpack_enc import refpack_compress
        body = refpack_compress(plaintext)
    else:
        body = refpack_compress_literals(plaintext)
    return b'EAR\x00' + struct.pack('<I', len(plaintext)) + body

if __name__ == '__main__':
    import sys
    p = sys.argv[1]
    plain = load_map(p)
    print(f'{p}: decompressed {len(plain)} bytes, head={plain[:4]}')
    open(p + '.plain', 'wb').write(plain)
    # round-trip check
    recomp = save_map(plain)
    plain2 = load_map_bytes = None
    chk = refpack_decompress(recomp[8:])
    print('roundtrip ok:', chk == plain)
