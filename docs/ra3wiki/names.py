# -*- coding: utf-8 -*-
"""Shared helper: turn a page title into a safe file name stem.

The local page files are named after the original yuque title. A few of those titles contain
characters Windows forbids in a path, so they are swapped for the full-width equivalents
(a full-width solidus reads as a slash but is a legal file-name character).

Used by work/yuque_fetch.py and docs/ra3wiki/rename_to_title.py, so both agree on the mapping.
"""
import re

ILLEGAL = {':': '：', '/': '／', '\\': '＼', '?': '？', '*': '＊',
           '"': '”', '<': '＜', '>': '＞', '|': '｜'}
RESERVED = {'CON', 'PRN', 'AUX', 'NUL', *(f'COM{i}' for i in range(1, 10)),
            *(f'LPT{i}' for i in range(1, 10))}


def safe_stem(title, fallback):
    """A Windows-safe file name stem for `title`, falling back to `fallback` (the slug)."""
    stem = ''.join(ILLEGAL.get(c, c) for c in str(title or ''))
    stem = re.sub(r'\s+', ' ', stem).strip().rstrip('.')
    if not stem:
        return fallback
    if stem.upper().split('.')[0] in RESERVED:
        stem = '_' + stem
    return stem[:120]
