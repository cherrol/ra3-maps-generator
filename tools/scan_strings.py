# -*- coding: utf-8 -*-
"""Scan a RA3 .map file for readable ASCII strings (format-independent recon)."""
import sys, re

def main(path, minlen=4):
    data = open(path, 'rb').read()
    print(f"file={path} size={len(data)}")
    # header hex
    print("head:", data[:64].hex(' '))
    pat = re.compile(rb'[\x20-\x7e]{%d,}' % minlen)
    seen = {}
    for m in pat.finditer(data):
        s = m.group().decode('ascii')
        seen.setdefault(s, m.start())
    # print in file order
    for s, off in sorted(seen.items(), key=lambda kv: kv[1]):
        print(f"{off:9d}  {s}")

if __name__ == '__main__':
    main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 4)
