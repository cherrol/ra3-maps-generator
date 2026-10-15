# -*- coding: utf-8 -*-
"""Recursive script tree dumper for RA3 PlayerScriptsList (Script v4/v5)."""
import sys, os, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ra3map import *

ASCII = re.compile(rb'[\x20-\x7e]{3,}')

def script_header(r, ver):
    name = r.s16()
    comments = [r.s16() for _ in range(3)]
    flags = [r.u8() for _ in range(6)]
    ev = r.u32() if ver >= 2 else 0
    if ver == 5:
        r.u8(); r.u32()
    if ver >= 3:
        seq_fire = r.u8(); loop = r.u8(); loopc = r.i32()
        stt = r.u8(); stn = r.s16()
    unk = r.s16() if ver >= 4 else ''
    return name, comments, flags, ev, unk

def walk(r, end, table, depth, out):
    while r.p < end:
        ni = r.u32(); ver = r.u16(); sz = r.u32()
        nm = table.get(ni, f'#{ni}')
        pend = r.p + sz
        pad = '  ' * depth
        if nm == 'Script':
            q = R(r.d, r.p)
            name, comments, flags, ev, unk = script_header(q, ver)
            strs = [s.decode() for s in ASCII.findall(r.d[r.p:pend])]
            extra = [s for s in strs if s not in (name,)]
            out.append(f'{pad}SCRIPT v{ver} name={name!r} flags={flags} hdr_end={q.p} params={extra}')
            walk(q, pend, table, depth + 1, out)
        elif nm == 'ScriptGroup':
            q = R(r.d, r.p)
            gname = q.s16()
            active, sub = q.u8(), q.u8()
            out.append(f'{pad}GROUP {gname!r} active={active} sub={sub}')
            walk(q, pend, table, depth + 1, out)
        elif nm in ('OrCondition', 'ScriptAction', 'ScriptActionFalse', 'ScriptCondition'):
            q = R(r.d, r.p)
            cver = q.u16()
            strs = [s.decode() for s in ASCII.findall(r.d[q.p:pend])]
            out.append(f'{pad}{nm} v{cver} params={strs}')
        else:
            out.append(f'{pad}{nm} v{ver} size={sz}')
        r.p = pend

def main(path):
    m = MapFile.load(path)
    c = m.get('PlayerScriptsList')
    r = R(c.payload)
    li = 0
    out = []
    while r.p < len(c.payload):
        ni = r.u32(); ver = r.u16(); sz = r.u32()
        nm = m.table.get(ni)
        end = r.p + sz
        out.append(f'=== ScriptList[{li}] {nm} v{ver} size={sz} ===')
        walk(R(r.d, r.p), end, m.table, 1, out)
        r.p = end
        li += 1
    txt = '\n'.join(out)
    open(path + '.scripts.txt', 'w', encoding='utf-8').write(txt)
    print(f'ScriptLists={li}, lines={len(out)} -> {path}.scripts.txt')

if __name__ == '__main__':
    main(sys.argv[1])
