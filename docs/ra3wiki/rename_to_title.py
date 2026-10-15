# -*- coding: utf-8 -*-
"""Rename every file in docs/ra3wiki/pages/ to the page's original title, and repoint all
references to it (the sheet note in the page itself, pages.json, catalog.json, catalog.md,
NAV.md).

Idempotent: a file already named after its title is left alone, so this is safe to re-run and
safe to run after a fresh fetch.

Usage:
    python docs/ra3wiki/rename_to_title.py --dry-run   # show the mapping only
    python docs/ra3wiki/rename_to_title.py             # apply
"""
import json
import pathlib
import re
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from names import safe_stem  # noqa: E402

sys.stdout.reconfigure(encoding='utf-8')

WIKI = pathlib.Path(__file__).resolve().parent
PAGES = WIKI / 'pages'


def main():
    dry = '--dry-run' in sys.argv
    index = json.loads((WIKI / 'pages.json').read_text(encoding='utf-8'))
    items = index['items']

    # the sheet sibling exists on disk, not necessarily in the index yet
    has_sheet = {p.name[:-len('.sheet.json')] for p in PAGES.glob('*.sheet.json')}

    # never touch a file the index does not know about. A `<page>.sheet.json` sibling of a known
    # page is fine even before the index has recorded it.
    page_names = {f'{e["slug"]}.md' for e in items} | \
                 {e['file'].split('/')[-1] for e in items if e.get('file')}
    stems = {n[:-len('.md')] for n in page_names}
    known = page_names | {f'{e["slug"]}.sheet.json' for e in items} | \
            {e['sheet'].split('/')[-1] for e in items if e.get('sheet')}
    extra = {p.name for p in PAGES.iterdir()
             if p.name not in known
             and not (p.name.endswith('.sheet.json') and p.name[:-len('.sheet.json')] in stems)}
    if extra:
        raise SystemExit(f'unexpected files in pages/, refusing to guess: {sorted(extra)}')

    targets = {}
    for e in items:
        stem = safe_stem(e['title'], e['slug'])
        targets[e['slug']] = (f'pages/{stem}.md',
                              f'pages/{stem}.sheet.json' if e['slug'] in has_sheet else '')
    dup = {}
    for slug, (f, _) in targets.items():
        dup.setdefault(f, []).append(slug)
    dup = {k: v for k, v in dup.items() if len(v) > 1}
    if dup:
        raise SystemExit(f'title collision, cannot rename 1:1: {dup}')

    # pass 1: move files. Two-phase through a temp name would only matter if a target collided
    # with another source, which the collision check above already rules out.
    renamed = 0
    for e in items:
        slug = e['slug']
        new_file, new_sheet = targets[slug]
        for old_rel, new_rel in ((f'pages/{slug}.md', new_file),
                                 (f'pages/{slug}.sheet.json', new_sheet)):
            if not new_rel:
                continue
            old = WIKI / old_rel
            new = WIKI / new_rel
            if old_rel == new_rel or not old.exists():
                continue
            if new.exists():
                raise SystemExit(f'target already exists: {new}')
            if not dry:
                old.rename(new)
            if old_rel.endswith('.md'):
                renamed += 1

        if dry:
            continue

        # the sheet note inside the page points at a sibling file; keep it in step
        if new_sheet:
            p = WIKI / new_file
            text = p.read_text(encoding='utf-8')
            fixed = re.sub(r'见 pages/.+?\.sheet\.json', f'见 {new_sheet}', text)
            if fixed != text:
                p.write_text(fixed, encoding='utf-8', newline='\n')

    if dry:
        for e in items:
            new_file, _ = targets[e['slug']]
            if e.get('file') != new_file:
                print(f"  {e['slug']:<20} {e.get('file') or '-':<28} -> {new_file}")
        print(f'would rename {renamed} page files')
        return

    # pass 2: repoint the index and every catalogue
    for e in items:
        new_file, new_sheet = targets[e['slug']]
        e['file'] = new_file
        # ask the disk, not the stale has_sheet snapshot: pass 1 has already moved the files
        new_sheet = f'pages/{safe_stem(e["title"], e["slug"])}.sheet.json'
        if (WIKI / new_sheet).exists():
            e['sheet'] = new_sheet
        else:
            e.pop('sheet', None)
    (WIKI / 'pages.json').write_text(
        json.dumps(index, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')

    by_slug = {e['slug']: e for e in items}

    cat = json.loads((WIKI / 'catalog.json').read_text(encoding='utf-8'))
    for it in cat['items']:
        e = by_slug.get(it.get('slug') or '')
        if not e:
            continue
        it['file'] = e['file']
        if e.get('sheet'):
            it['sheet'] = e['sheet']
        else:
            it.pop('sheet', None)
    (WIKI / 'catalog.json').write_text(
        json.dumps(cat, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')

    for name in ('catalog.md', 'NAV.md'):
        text = (WIKI / name).read_text(encoding='utf-8')
        for slug, e in by_slug.items():
            text = text.replace(f'](pages/{slug}.md)', f']({e["file"]})')
        (WIKI / name).write_text(text, encoding='utf-8', newline='\n')

    print(f'renamed {renamed} page files; pages.json, catalog.json, catalog.md, NAV.md updated')


if __name__ == '__main__':
    main()
