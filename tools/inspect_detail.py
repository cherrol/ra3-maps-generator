# -*- coding: utf-8 -*-
"""Detail dump: WorldInfo props, ScriptList structure, selected objects."""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *

def dump_worldinfo(m):
    c = m.get('WorldInfo')
    r = R(c.payload)
    props = read_props(r, m.table)
    print('WorldInfo v%d consumed=%d/%d' % (c.version, r.p, len(c.payload)))
    for k, t, v in props:
        print('   ', repr(k), 'type', t, '=', repr(v)[:200])

def dump_scripts(m):
    c = m.get('PlayerScriptsList')
    r = R(c.payload)
    print('PlayerScriptsList v%d size=%d' % (c.version, len(c.payload)))
    li = 0
    while r.p < len(c.payload):
        nidx = r.u32(); ver = r.u16(); size = r.u32()
        nm = m.table.get(nidx)
        end = r.p + size
        print(f'  ScriptList[{li}] name={nm} v{ver} size={size}')
        # inner
        rr = R(r.d, r.p)
        while rr.p < end:
            ni = rr.u32(); v2 = rr.u16(); sz = rr.u32()
            nm2 = m.table.get(ni)
            print(f'      {nm2} v{v2} size={sz}')
            payload = rr.d[rr.p:rr.p+sz]
            if nm2 == 'Script':
                # parse header per spec
                q = R(payload)
                name = q.s16(); c1 = q.s16(); c2 = q.s16(); c3 = q.s16()
                flags = [q.u8() for _ in range(6)]
                ev = q.u32()
                uit = q.u8(); itype = q.u32()
                seq = [q.u8(), q.u8()]
                loopc = q.i32(); stt = q.u8(); stn = q.s16(); unk = q.s16()
                print(f'         name={name!r} comments={c1!r},{c2!r},{c3!r} flags={flags} ev={ev} uit={uit} itype={itype} seq={seq} loop={loopc} stt={stt} stn={stn!r} unk={unk!r} pos={q.p}/{sz}')
            rr.p += sz
        r.p = end
        li += 1

def dump_objects_detail(m, want):
    objs = parse_objects(m.get('ObjectsList'), m.table)
    for o in objs:
        if o['type_name'] in want:
            d = props_dict(o['props'])
            print(f"{o['type_name']:35s} pos={tuple(round(x) for x in o['position'])} angle={o['angle']:.2f} {d}")

if __name__ == '__main__':
    m = MapFile.load(sys.argv[1])
    dump_worldinfo(m)
    dump_scripts(m)
    dump_objects_detail(m, {'SovietConstructionYard', 'AlliedConstructionYard', 'OreNode', 'OilDerrick', 'ObservationPostTechStructure', 'GarageTechStructure'})
