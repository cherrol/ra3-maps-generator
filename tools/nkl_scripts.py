# -*- coding: utf-8 -*-
"""把另一张图公共列表里的脚本接到当前图的公共列表后面。

脚本块的名字编号属于来源图的字符串表。接入时按名字改到目标图的编号，
缺的名字补进目标图的表。脚本正文里的玩家名、物体名不改。
"""
import struct

MAGIC = b'NKL1'
_CONTAINERS = ('Script', 'ScriptGroup', 'OrCondition')
_LEAVES = ('ScriptAction', 'ScriptActionFalse', 'Condition')


def _u16str(d, p):
    n = struct.unpack_from('<H', d, p)[0]
    return d[p + 2:p + 2 + n].decode('latin-1'), p + 2 + n


def _script_children(d, b, ver):
    p = b
    for _ in range(4):
        _, p = _u16str(d, p)
    p += 6
    if ver >= 2:
        p += 4
        if ver == 5:
            p += 5
    if ver >= 3:
        p += 7
        _, p = _u16str(d, p)
    if ver >= 4:
        _, p = _u16str(d, p)
    if ver >= 6:
        p += 6
    return p


def _group_children(d, b):
    _, p = _u16str(d, b)
    return p + 2


def _walk(d, start, end, table, on_asset):
    p = start
    while p + 10 <= end:
        ni = struct.unpack_from('<I', d, p)[0]
        ver = struct.unpack_from('<H', d, p + 4)[0]
        sz = struct.unpack_from('<I', d, p + 6)[0]
        b = p + 10
        pend = b + sz
        if sz < 0 or pend > end:
            raise SystemExit('脚本块越界')
        nm = table[ni]
        on_asset(p, ni, ver, b, pend, nm)
        if nm == 'Script':
            _walk(d, _script_children(d, b, ver), pend, table, on_asset)
        elif nm == 'ScriptGroup':
            _walk(d, _group_children(d, b), pend, table, on_asset)
        elif nm == 'OrCondition':
            _walk(d, b, pend, table, on_asset)
        elif nm not in _LEAVES:
            raise SystemExit('不认识的脚本块：%s' % nm)
        p = pend
    if p != end:
        raise SystemExit('脚本块末尾多出 %d 字节' % (end - p))


def _internal_index(d, b, pend, nm, ver):
    need = (nm in ('ScriptAction', 'ScriptActionFalse') and ver >= 2) or (
        nm == 'Condition' and ver >= 4)
    if not need:
        return None
    if pend - b < 8:
        raise SystemExit('%s 太短，没有内部名' % nm)
    return d[b + 5] | (d[b + 6] << 8) | (d[b + 7] << 16)


def first_script_list_body(mmap):
    """公共列表（第 0 个 ScriptList）的正文。"""
    chunk = mmap.get('PlayerScriptsList')
    data = chunk.payload
    if len(data) < 10:
        raise SystemExit('这张图没有脚本列表')
    sz = struct.unpack_from('<I', data, 6)[0]
    return data[10:10 + sz]


def pack_script_list(src_map, out_path):
    """把来源图的公共脚本打成一份可嵌入的文件。"""
    body = first_script_list_body(src_map)
    table = src_map.table
    names = {}

    def on_asset(p, ni, ver, b, pend, nm):
        names[ni] = table[ni]
        inn = _internal_index(body, b, pend, nm, ver)
        if inn is not None:
            names[inn] = table[inn]

    _walk(body, 0, len(body), table, on_asset)
    w = bytearray(MAGIC)
    w += struct.pack('<I', len(names))
    for idx, name in names.items():
        raw = name.encode('latin-1')
        w += struct.pack('<IH', idx, len(raw))
        w += raw
    w += struct.pack('<I', len(body))
    w += body
    open(out_path, 'wb').write(w)
    return len(body), len(names)


def _load_pack(path):
    data = open(path, 'rb').read()
    if data[:4] != MAGIC:
        raise SystemExit('不是 NKL 脚本包：' + path)
    n = struct.unpack_from('<I', data, 4)[0]
    p = 8
    table = {}
    for _ in range(n):
        idx, ln = struct.unpack_from('<IH', data, p)
        p += 6
        table[idx] = data[p:p + ln].decode('latin-1')
        p += ln
    blen = struct.unpack_from('<I', data, p)[0]
    p += 4
    body = data[p:p + blen]
    if len(body) != blen:
        raise SystemExit('NKL 脚本包被截断')
    return table, body


def _ensure_names(dst, needed):
    have = {}
    for name, idx in dst.order:
        have.setdefault(name, idx)
    nxt = max(idx for _, idx in dst.order) + 1
    added = []
    idmap = {}
    for name in needed:
        if name not in have:
            have[name] = nxt
            dst.order.append((name, nxt))
            dst.table[nxt] = name
            added.append(name)
            nxt += 1
        idmap[name] = have[name]
    return idmap, added


def _remap(body, src_table, idmap):
    out = bytearray()

    def on_asset(p, ni, ver, b, pend, nm):
        out.extend(struct.pack('<I', idmap[src_table[ni]]))
        out.extend(body[p + 4:b])
        if nm == 'Script':
            child = _script_children(body, b, ver)
            out.extend(body[b:child])
        elif nm == 'ScriptGroup':
            child = _group_children(body, b)
            out.extend(body[b:child])
        elif nm == 'OrCondition':
            return
        elif nm in _LEAVES:
            chunk = bytearray(body[b:pend])
            inn = _internal_index(body, b, pend, nm, ver)
            if inn is not None:
                nid = idmap[src_table[inn]]
                chunk[5] = nid & 0xFF
                chunk[6] = (nid >> 8) & 0xFF
                chunk[7] = (nid >> 16) & 0xFF
            out.extend(chunk)
        else:
            raise SystemExit('不认识的脚本块：%s' % nm)

    _walk(body, 0, len(body), src_table, on_asset)
    if len(out) != len(body):
        raise SystemExit('改编号后脚本长度变了：%d -> %d' % (len(body), len(out)))
    _walk(bytes(out), 0, len(out), {i: n for n, i in idmap.items()}, lambda *a: None)
    return bytes(out)


def append_nkl_scripts(dst_map, pack_path):
    """把脚本包接到目标图公共列表的末尾。"""
    src_table, body = _load_pack(pack_path)
    idmap, added = _ensure_names(dst_map, set(src_table.values()))
    extra = _remap(body, src_table, idmap)
    chunk = dst_map.get('PlayerScriptsList')
    data = chunk.payload
    sz = struct.unpack_from('<I', data, 6)[0]
    if 10 + sz > len(data):
        raise SystemExit('公共脚本列表长度不对')
    new_body = data[10:10 + sz] + extra
    chunk.payload = (
        data[:6] + struct.pack('<I', len(new_body)) + new_body + data[10 + sz:]
    )
    # 接好的整表还要能按块走完
    _walk(new_body, 0, len(new_body), dst_map.table, lambda *a: None)
    print('  [脚本] 接入 NKL 公共脚本 %d 字节，新名字 %d 个' % (len(extra), len(added)))
