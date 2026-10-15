# -*- coding: utf-8 -*-
"""Regenerate docs/ra3wiki/NAV.md: the book TOC with local file links where a page was fetched.

Run after yuque_fetch.py so the local links match what is on disk.
"""
import json
import pathlib
import sys

sys.stdout.reconfigure(encoding='utf-8')

ROOT = pathlib.Path(r'D:\workspace\ra3maps')
WIKI = ROOT / 'docs' / 'ra3wiki'


def main():
    cat = json.loads((WIKI / 'catalog.json').read_text(encoding='utf-8'))
    idx_path = WIKI / 'pages.json'
    fetched = {}
    if idx_path.exists():
        for e in json.loads(idx_path.read_text(encoding='utf-8'))['items']:
            fetched[e['slug']] = e

    num = 0
    out = [
        '# ra3wiki 本地目录',
        '',
        f'目录结构与 [语雀原文]({cat["source"]}) 一致；已抓取的页面直接链到本地文件，',
        '未抓取的（地编教程）保留原文链接。索引见 `pages.json`，文件在 `pages/`。',
        '',
    ]
    local = 0
    skipped = []
    # indentation follows the TITLE nesting: a DOC sits one level inside every section heading
    # that precedes it, which keeps the flat `path` values readable as a tree.
    stack = []  # [(level, title)]
    for it in cat['items']:
        level = it.get('level', 0) or 0
        title = it['title']
        while stack and stack[-1][0] >= level:
            stack.pop()
        if it.get('type') != 'DOC' or not it.get('slug'):
            indent = '  ' * len(stack)
            out.append(f'{indent}- **{title}**')
            stack.append((level, title))
            continue
        num += 1
        slug = it['slug']
        indent = '  ' * len(stack)
        if slug in fetched:
            e = fetched[slug]
            if e.get('format') == 'lakesheet':
                note = '（表格页）'
            elif not e.get('chars', 0):
                note = '（原文正文为空）'
            else:
                note = ''
            rel = e.get('file') or f'pages/{slug}.md'
            out.append(f'{indent}- [{num:3d}. {title}]({rel}) {note}'
                       f' · [原文]({it["href"]})')
            local += 1
        else:
            out.append(f'{indent}- [{num:3d}. {title}]({it["href"]}) · 未抓取')
            skipped.append(title)

    out += [
        '',
        f'共 {num} 个文档条目，本地已抓 {local} 个，' + f'未抓 {len(skipped)} 个（地编教程系列）。',
        '',
    ]
    (WIKI / 'NAV.md').write_text('\n'.join(out), encoding='utf-8', newline='\n')
    print(f'NAV.md written: {local} local / {len(skipped)} not fetched')


if __name__ == '__main__':
    main()
