# -*- coding: utf-8 -*-
"""RA3 .map reader/writer library (SAGE CkMp format).

Reference: OpenSAGE sources (Data/Map/*.cs) + docs/ra3_map_format_spec.md
"""
import struct, sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from refpack import load_map, save_map

# ---------------------------------------------------------------- byte reader
class R:
    def __init__(self, data, pos=0):
        self.d = data; self.p = pos
    def u8(self):
        v = self.d[self.p]; self.p += 1; return v
    def u16(self):
        v, = struct.unpack_from('<H', self.d, self.p); self.p += 2; return v
    def u24(self):
        v = self.d[self.p] | (self.d[self.p+1] << 8) | (self.d[self.p+2] << 16); self.p += 3; return v
    def u32(self):
        v, = struct.unpack_from('<I', self.d, self.p); self.p += 4; return v
    def i32(self):
        v, = struct.unpack_from('<i', self.d, self.p); self.p += 4; return v
    def f32(self):
        v, = struct.unpack_from('<f', self.d, self.p); self.p += 4; return v
    def vec3(self):
        return (self.f32(), self.f32(), self.f32())
    def vec2(self):
        return (self.f32(), self.f32())
    def s16(self):
        n = self.u16(); s = self.d[self.p:self.p+n].decode('latin-1'); self.p += n; return s
    def u16str(self):
        return self.s16()
    def varint_str(self):
        shift = 0; ln = 0
        while True:
            b = self.d[self.p]; self.p += 1
            ln |= (b & 0x7F) << shift
            if not (b & 0x80): break
            shift += 7
        s = self.d[self.p:self.p+ln].decode('ascii'); self.p += ln
        return s
    def rest(self):
        return self.d[self.p:]

class W:
    def __init__(self):
        self.b = bytearray()
    def u8(self, v): self.b.append(v & 0xFF)
    def u16(self, v): self.b += struct.pack('<H', v)
    def u24(self, v): self.b += bytes((v & 0xFF, (v >> 8) & 0xFF, (v >> 16) & 0xFF))
    def u32(self, v): self.b += struct.pack('<I', v)
    def i32(self, v): self.b += struct.pack('<i', v)
    def f32(self, v): self.b += struct.pack('<f', v)
    def vec3(self, v):
        for x in v: self.f32(x)
    def s16(self, s):
        bs = s.encode('latin-1') if isinstance(s, str) else s
        self.u16(len(bs)); self.b += bs
    def varint_str(self, s):
        bs = s.encode('ascii')
        ln = len(bs)
        while True:
            b = ln & 0x7F; ln >>= 7
            if ln: self.b.append(b | 0x80)
            else: self.b.append(b); break
        self.b += bs

# ---------------------------------------------------------------- properties
PROP_BOOL, PROP_INT, PROP_FLOAT, PROP_ASCII, PROP_UNICODE, PROP_UNKNOWN = 0, 1, 2, 3, 4, 5

def read_props(r, table):
    n = r.u16()
    props = []
    for _ in range(n):
        t = r.u8(); key = table[r.u24()]
        if t == PROP_BOOL:
            v = r.u8()
        elif t == PROP_INT:
            v = r.i32()
        elif t == PROP_FLOAT:
            v = r.f32()
        elif t in (PROP_ASCII, PROP_UNKNOWN):
            v = r.u16str()
        elif t == PROP_UNICODE:
            ln = r.u16(); v = r.d[r.p:r.p+ln*2].decode('utf-16-le'); r.p += ln*2
        else:
            raise ValueError(f'unknown property type {t} for {key}')
        props.append([key, t, v])
    return props

def write_props(w, props, key_ids):
    w.u16(len(props))
    for key, t, v in props:
        w.u8(t)
        w.u24(key if isinstance(key, int) else key_ids[key])
        if t == PROP_BOOL:
            w.u8(1 if v else 0)
        elif t == PROP_INT:
            w.i32(v)
        elif t == PROP_FLOAT:
            w.f32(v)
        elif t in (PROP_ASCII, PROP_UNKNOWN):
            w.s16(v)
        elif t == PROP_UNICODE:
            bs = v.encode('utf-16-le'); w.u16(len(bs)//2); w.b += bs

def props_dict(props):
    return {k: v for k, t, v in props}

# ---------------------------------------------------------------- container
class Chunk:
    def __init__(self, name, name_idx, version, payload=b''):
        self.name = name; self.name_idx = name_idx
        self.version = version; self.payload = payload
    def __repr__(self):
        return f'<Chunk {self.name} v{self.version} {len(self.payload)}B>'

def read_string_table(r):
    n = r.u32()
    table = {}
    order = []
    for _ in range(n):
        s = r.varint_str()
        idx = r.u32()
        table[idx] = s
        order.append((s, idx))
    return table, order

def write_string_table(w, order):
    w.u32(len(order))
    for s, idx in order:
        w.varint_str(s)
        w.u32(idx)

class MapFile:
    def __init__(self):
        self.order = []          # [(name, id)] string table, in file order
        self.table = {}          # id -> name
        self.chunks = []         # [Chunk]
        self.raw_plain = None

    # ---- io
    @classmethod
    def load(cls, path):
        return cls.from_bytes(load_map(path))

    @classmethod
    def from_bytes(cls, data):
        assert data[:4] == b'CkMp', data[:4]
        self = cls()
        self.raw_plain = data
        r = R(data, 4)
        self.table, self.order = read_string_table(r)
        while r.p < len(data):
            name_idx = r.u32(); ver = r.u16(); size = r.u32()
            name = self.table.get(name_idx, f'#{name_idx}')
            payload = data[r.p:r.p+size]; r.p += size
            self.chunks.append(Chunk(name, name_idx, ver, payload))
        return self

    def to_bytes(self):
        w = W()
        w.b += b'CkMp'
        write_string_table(w, self.order)
        for c in self.chunks:
            w.u32(c.name_idx); w.u16(c.version); w.u32(len(c.payload)); w.b += c.payload
        return bytes(w.b)

    def save(self, path, compress=True):
        data = self.to_bytes()
        open(path, 'wb').write(save_map(data) if compress else data)

    def get(self, name):
        for c in self.chunks:
            if c.name == name:
                return c
        return None
    def all(self, name):
        return [c for c in self.chunks if c.name == name]

# ---------------------------------------------------------------- specific chunks
def parse_heightmap(chunk):
    r = R(chunk.payload)
    d = {}
    d['width'] = r.u32(); d['height'] = r.u32()
    d['border_width'] = r.u32(); d['border_count'] = r.u32()
    borders = []
    for _ in range(d['border_count']):
        b = [r.u32(), r.u32(), r.u32(), r.u32()]
        borders.append(b)
    d['borders'] = borders
    d['area'] = r.u32()
    assert d['area'] == d['width'] * d['height'], (d['area'], d['width'], d['height'])
    n = d['area']
    d['elevations'] = list(struct.unpack_from('<%dH' % n, r.d, r.p)); r.p += 2*n
    d['consumed'] = r.p
    return d

def parse_blendtiles(chunk, width, height, table):
    r = R(chunk.payload)
    ver = chunk.version
    d = {'version': ver}
    d['num_tiles'] = r.u32()
    n = d['num_tiles']
    d['tiles'] = list(struct.unpack_from('<%dH' % n, r.d, r.p)); r.p += 2*n
    wide = 14 <= ver < 24
    fmt = '<%dI' % n if wide else '<%dH' % n
    sz = 4 if wide else 2
    for key in ('blends', 'threeway', 'cliff_tex'):
        d[key] = list(struct.unpack_from(fmt, r.d, r.p)); r.p += sz*n
    # bit arrays
    def bits(rows, cols):
        out = []
        rowbytes = (cols + 7)//8
        for y in range(rows):
            row = r.d[r.p:r.p+rowbytes]; r.p += rowbytes
            out.append([(row[x >> 3] >> (x & 7)) & 1 for x in range(cols)])
        return out
    if ver > 6:  d['impassability'] = bits(height, width)
    if ver >= 10: d['impassability_to_players'] = bits(height, width)
    if ver >= 11: d['passage_widths'] = bits(height, width)
    if 14 <= ver < 25: d['taintability'] = bits(height, width)
    if ver >= 15: d['extra_passability'] = bits(height, width)
    if 16 <= ver < 25:
        d['flammability'] = list(r.d[r.p:r.p+n]); r.p += n
    if ver >= 17: d['visibility'] = bits(height, width)
    order_note = None
    if ver >= 24:
        d['buildability'] = bits(height, width)
        d['impassability_to_air'] = bits(height, width)
        d['tiberium_growability'] = bits(height, width)
    if ver >= 25:
        d['shrubbery_density'] = list(r.d[r.p:r.p+n]); r.p += n
    d['texture_cell_count'] = r.u32()
    bc = r.u32(); d['blends_count_raw'] = bc; d['blends_count'] = bc - 1 if bc > 0 else 0
    cc = r.u32(); d['cliff_count_raw'] = cc; d['cliff_count'] = cc - 1 if cc > 0 else 0
    tc = r.u32(); d['texture_count'] = tc
    texs = []
    for _ in range(tc):
        t = {}
        t['cell_start'] = r.u32(); t['cell_count'] = r.u32()
        t['cell_size'] = r.u32(); t['magic'] = r.u32(); t['name'] = r.u16str()
        texs.append(t)
    d['textures'] = texs
    d['magic1'] = r.u32(); d['magic2'] = r.u32()
    blends = []
    for _ in range(d['blends_count']):
        b = {}
        b['secondary_tile'] = r.u32()
        b['direction'] = list(r.d[r.p:r.p+4]); r.p += 4
        b['flags'] = r.u8(); b['two_sided'] = r.u8()
        b['magic1'] = r.u32(); b['magic2'] = r.u32()
        blends.append(b)
    d['blend_descriptions'] = blends
    cliffs = []
    for _ in range(d['cliff_count']):
        c = {}
        c['texture_tile'] = r.u32()
        c['bl'] = r.vec2(); c['br'] = r.vec2(); c['tr'] = r.vec2(); c['tl'] = r.vec2()
        c['unknown2'] = r.u16()
        cliffs.append(c)
    d['cliff_mappings'] = cliffs
    d['consumed'] = r.p
    return d

def parse_objects(chunk, table):
    r = R(chunk.payload)
    objs = []
    while r.p < len(chunk.payload):
        name_idx = r.u32(); ver = r.u16(); size = r.u32()
        nm = table.get(name_idx, f'#{name_idx}')
        end = r.p + size
        assert nm == 'Object', f'unexpected subchunk {nm}'
        o = {'version': ver}
        o['position'] = r.vec3()
        o['angle'] = r.f32()
        o['road_type'] = r.u32()
        o['type_name'] = r.u16str()
        o['props'] = read_props(r, table)
        assert r.p == end, f'object parse mismatch {r.p} != {end}'
        objs.append(o)
    return objs

def write_objects(objs, name_idx_object, key_ids):
    w = W()
    for o in objs:
        body = W()
        body.vec3(o['position']); body.f32(o['angle']); body.u32(o['road_type'])
        body.s16(o['type_name']); write_props(body, o['props'], key_ids)
        w.u32(name_idx_object); w.u16(o['version']); w.u32(len(body.b)); w.b += body.b
    return bytes(w.b)

def parse_sides(chunk, table, has_asset_list=True):
    r = R(chunk.payload)
    d = {'version': chunk.version}
    if chunk.version >= 6:
        d['unknown'] = r.u8()
    n = r.i32(); d['num_players'] = n
    players = []
    for _ in range(n):
        p = {}
        p['props'] = read_props(r, table)
        cnt = r.u32()
        items = []
        for _ in range(cnt):
            it = {}
            it['building_name'] = r.u16str()
            it['template_name'] = r.u16str()
            it['location'] = r.vec3()
            it['angle'] = r.f32()
            it['initially_built'] = r.u8()
            if has_asset_list:
                it['unknown1'] = r.u8()
            it['num_rebuilds'] = r.u32()
            it['script'] = r.u16str()
            it['health'] = r.i32()
            it['whiner'] = r.u8(); it['unsellable'] = r.u8(); it['repairable'] = r.u8()
            items.append(it)
        p['build_list'] = items
        players.append(p)
    d['players'] = players
    d['consumed'] = r.p
    return d

def parse_teams(chunk, table):
    r = R(chunk.payload)
    n = r.i32()
    teams = [read_props(r, table) for _ in range(n)]
    return teams

def parse_waypointslist(chunk):
    r = R(chunk.payload)
    n = r.u32()
    paths = [(r.i32(), r.i32()) for _ in range(n)]
    return paths

def parse_mp_positions(chunk, table):
    r = R(chunk.payload)
    out = []
    while r.p < len(chunk.payload):
        name_idx = r.u32(); ver = r.u16(); size = r.u32()
        end = r.p + size
        e = {'version': ver, 'name': table.get(name_idx, f'#{name_idx}')}
        e['is_human'] = r.u8(); e['is_computer'] = r.u8()
        if ver > 0:
            e['load_ai_script'] = r.u8(); e['team'] = r.u32()
            cnt = r.u32()
            e['side_restrictions'] = [r.u16str() for _ in range(cnt)]
        assert r.p == end, f'mp chunk {r.p} != {end}'
        out.append(e)
    return out

if __name__ == '__main__':
    m = MapFile.load(sys.argv[1])
    print('chunks:', len(m.chunks))
    for c in m.chunks:
        print(' ', c)
