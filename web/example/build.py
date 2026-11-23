#!/usr/bin/env python3
"""Build the web example.

  python3 web/example/build.py dev
      dist/web/: index.html, qvp.js, app.js, qvp_ffi.wasm, a link to dist/pages, and a link
      to dist/letters when it is there.
      Serve that directory: (cd dist/web && python3 -m http.server 8765), open /?p=42
  python3 web/example/build.py embed 1-21,440-445,582,604
      dist/demo.html: one file with the engine and the listed pages inlined.

Inputs: web/qvp.js, web/qvp_ffi.wasm (cargo build for wasm32, then copy), dist/pages/, and
the letter layers in dist/letters/ when they are there (docs/DATA.md). QVP_PAGES_DIR and
QVP_LETTERS_DIR name other directories. A layer attaches only to the page file it was built
with, so take the pages and the layers from the same build.
"""
import base64, hashlib, json, os, shutil, sys

root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
web = os.path.join(root, 'web'); example = os.path.join(web, 'example')
dist = os.path.join(root, 'dist')
pages_dir = os.path.abspath(os.environ.get('QVP_PAGES_DIR') or os.path.join(dist, 'pages'))
letters_dir = os.path.abspath(os.environ.get('QVP_LETTERS_DIR') or os.path.join(dist, 'letters'))


def link(target, at):
    """Point `at` to `target`, in the place of what was there. With no target, leave nothing."""
    if os.path.islink(at):
        os.remove(at)
    elif os.path.exists(at):
        shutil.rmtree(at)
    if os.path.isdir(target):
        os.symlink(target, at)
frag = open(os.path.join(example, 'app.html')).read()
qvp = open(os.path.join(web, 'qvp.js')).read()
app = open(os.path.join(example, 'app.js')).read()
mode = sys.argv[1] if len(sys.argv) > 1 else 'dev'


def pages_arg(s):
    out = []
    for part in s.split(','):
        a, _, b = part.partition('-')
        out.extend(range(int(a), int(b or a) + 1))
    return out


if mode == 'dev':
    out_dir = os.path.join(dist, 'web'); os.makedirs(out_dir, exist_ok=True)
    # The scripts carry a digest of this build in their address. A browser that kept an older
    # script then asks for the new one, so the page and its scripts are always of one build.
    build = hashlib.sha256((frag + qvp + app).encode()).hexdigest()[:10]
    html = ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width,initial-scale=1">' + frag +
            f'<script src="qvp.js?v={build}"></script><script src="app.js?v={build}"></script></html>')
    open(os.path.join(out_dir, 'index.html'), 'w').write(html)
    for name, src in (('qvp.js', web), ('app.js', example), ('qvp_ffi.wasm', web)):
        shutil.copy(os.path.join(src, name), os.path.join(out_dir, name))
    link(pages_dir, os.path.join(out_dir, 'pages'))
    link(letters_dir, os.path.join(out_dir, 'letters'))
    print('dist/web/ written; serve it: (cd dist/web && python3 -m http.server 8765)')
else:
    pages = pages_arg(sys.argv[2] if len(sys.argv) > 2 else '1-21,440-445,582,604')
    wasm = base64.b64encode(open(os.path.join(web, 'qvp_ffi.wasm'), 'rb').read()).decode()
    emb = {'wasm': wasm, 'pages': {}, 'words': {}, 'letters': {}}
    ap = os.path.join(pages_dir, 'atlas.qva')
    if os.path.exists(ap): emb['atlas'] = base64.b64encode(open(ap, 'rb').read()).decode()
    op = os.path.join(pages_dir, 'ornaments.qvo')
    if os.path.exists(op): emb['ornaments'] = base64.b64encode(open(op, 'rb').read()).decode()
    for n in pages:
        k = f'{n:03d}'
        emb['pages'][k] = base64.b64encode(open(os.path.join(pages_dir, k + '.qvp'), 'rb').read()).decode()
        emb['words'][k] = json.load(open(os.path.join(pages_dir, k + '.words.json')))
        lp = os.path.join(letters_dir, k + '.qvl')
        if os.path.exists(lp): emb['letters'][k] = base64.b64encode(open(lp, 'rb').read()).decode()
    payload = json.dumps(emb, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
    html = frag + '\n<script>window.QVP_EMBED=' + payload + ';</script>\n<script>' + qvp + '</script>\n<script>' + app + '</script>\n'
    os.makedirs(dist, exist_ok=True)
    out = os.path.join(dist, 'demo.html')
    open(out, 'w').write(html)
    print(f'{out}: {os.path.getsize(out)/1e6:.2f} MB, {len(pages)} pages embedded')
