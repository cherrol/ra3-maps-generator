# -*- coding: utf-8 -*-
"""Point the catalogue at the local copies of the pages that were downloaded.

* `catalog.json`: every DOC entry that exists locally gets a `file` field
  (`pages/<slug>.md`, or `.sheet.json` for 表格页). The `href` field stays on the yuque URL
  because `work/yuque_fetch.py` reads it as the crawl source.
* `catalog.md`: entries that exist locally link to the local file; the rest keep the original
  yuque link. Downloaded entries stay plain text, with a note when the original body is empty.

Run after yuque_fetch.py. Usage: python docs/ra3wiki/link_catalog.py
"""
import json
import pathlib
import sys

sys.stdout.reconfigure(encoding='utf-8')

ROOT = pathlib.Path(r'D:\workspace\ra3maps')
WIKI = ROOT / 'docs' / 'ra3wiki'


def load_fetched():
    p = WIKI / 'pages.json'
    if not p.exists():
        raise SystemExit('pages.json not found; run work/yuque_fetch.py first')
    return {e['slug']: e for e in json.loads(p.read_text(encoding='utf-8'))['items']}


def local_path(entry, fetched):
    """The local file for a catalogue entry: whatever the fetch index recorded.

    Run rename_to_title.py first; it is what keeps `file` in step with the real file names.
    """
    e = fetched.get(entry['slug'])
    if not e:
        return None, None
    f = e.get('file') or f'pages/{entry["slug"]}.md'
    return f, (e.get('sheet') or None)


def note_for(entry, fetched):
    """A short marker for the markdown catalogue, '' when nothing needs saying."""
    e = fetched.get(entry['slug'])
    if not e:
        return '未下载'
    if e.get('format') == 'lakesheet':
        return '表格页'
    if not e.get('chars', 0):
        return '原文正文为空'
    return ''


def check_files(cat, fetched):
    missing = []
    for it in cat['items']:
        f, sh = local_path(it, fetched)
        for rel in (f, sh):
            if rel and not (WIKI / rel).exists():
                missing.append(rel)
    if missing:
        raise SystemExit(
            'these index paths do not exist on disk, run rename_to_title.py first:\n  '
            + '\n  '.join(sorted(set(missing))))


def rewrite_json(cat, fetched):
    n = 0
    for it in cat['items']:
        slug = it.get('slug')
        if it.get('type') != 'DOC' or not slug or slug not in fetched:
            it.pop('file', None)
            it.pop('sheet', None)
            continue
        f, sh = local_path(it, fetched)
        it['file'] = f
        if sh:
            it['sheet'] = sh
        else:
            it.pop('sheet', None)
        n += 1
    cat['local'] = {
        'count': n,
        'note': 'file 是本地文件（相对本目录，即页面原标题）；href 保留语雀原文地址，'
                '供 yuque_fetch.py 重新抓取。',
    }
    return n


def rewrite_md(cat, fetched):
    out = [
        f'# {cat.get("book") or "红色警戒3 地图编辑器 wiki"}',
        '',
        f'目录来自 [{cat["source"]}]({cat["source"]})，抓取日期 {cat["fetched"]}。',
        '已下载到本地的条目链到本地文件，未下载的链回原文。',
        '本地文件在 `pages/`，索引见 `pages.json`，可读目录见 `NAV.md`。',
        '',
    ]
    for it in cat['items']:
        level = it.get('level', 0) or 0
        title = it['title']
        if it.get('type') != 'DOC' or not it.get('slug'):
            if out and out[-1] != '':
                out.append('')
            out.append(f'{"##" if level == 0 else "###"} {title}')
            continue
        slug = it['slug']
        n = note_for(it, fetched)
        tail = f' · {n}' if n else ''
        f, _ = local_path(it, fetched)
        if f:
            out.append(f'- [{title}]({f}){tail}')
        else:
            out.append(f'- [{title}]({it["href"]}){tail}')
    out.append('')
    done = sum(1 for it in cat['items']
               if it.get('type') == 'DOC' and it.get('slug') in fetched)
    total = sum(1 for it in cat['items'] if it.get('type') == 'DOC')
    out += [
        f'共 {total} 个文档条目，本地已下载 {done} 个，未下载 {total - done} 个。',
        '',
        '本地文件名就是页面原标题；表格页（语雀 `lakesheet`）的正文已解压成 markdown 表格，',
        '原始表格数据另存为同名 `.sheet.json`，`catalog.json` 里对应条目有 `sheet` 字段指向它。',
        '',
    ]
    return '\n'.join(out)


def main():
    fetched = load_fetched()
    cat = json.loads((WIKI / 'catalog.json').read_text(encoding='utf-8'))
    check_files(cat, fetched)

    n = rewrite_json(cat, fetched)
    (WIKI / 'catalog.json').write_text(
        json.dumps(cat, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    (WIKI / 'catalog.md').write_text(rewrite_md(cat, fetched), encoding='utf-8', newline='\n')
    print(f'catalog.json: {n} entries linked to local files')
    print('catalog.md: rewritten')


if __name__ == '__main__':
    main()
