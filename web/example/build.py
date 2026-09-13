#!/usr/bin/env python3
"""Build the web example.

  python3 web/example/build.py dev
      dist/web/: index.html, qvp.js, app.js, qvp_ffi.wasm and a link to dist/pages.
      Serve that directory: (cd dist/web && python3 -m http.server 8765), open /?p=42
  python3 web/example/build.py embed 1-21,440-445,582,604
      dist/demo.html: one file with the engine and the listed pages inlined.

Inputs: web/qvp.js, web/qvp_ffi.wasm (cargo build for wasm32, then copy), dist/pages/.
"""
import base64, json, os, shutil, sys

root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
web = os.path.join(root, 'web'); example = os.path.join(web, 'example')
dist = os.path.join(root, 'dist'); pages_dir = os.path.join(dist, 'pages')
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
    html = ('<!doctype html><html lang="en"><head><meta charset="utf-8">'
            '<meta name="viewport" content="width=device-width,initial-scale=1">' + frag +
            '<script src="qvp.js"></script><script src="app.js"></script></html>')
    open(os.path.join(out_dir, 'index.html'), 'w').write(html)
    for name, src in (('qvp.js', web), ('app.js', example), ('qvp_ffi.wasm', web)):
        shutil.copy(os.path.join(src, name), os.path.join(out_dir, name))
    link = os.path.join(out_dir, 'pages')
    if os.path.islink(link) or os.path.exists(link):
        os.remove(link) if os.path.islink(link) else shutil.rmtree(link)
    os.symlink(pages_dir, link)
    print('dist/web/ written; serve it: (cd dist/web && python3 -m http.server 8765)')
else:
    pages = pages_arg(sys.argv[2] if len(sys.argv) > 2 else '1-21,440-445,582,604')
    wasm = base64.b64encode(open(os.path.join(web, 'qvp_ffi.wasm'), 'rb').read()).decode()
    emb = {'wasm': wasm, 'pages': {}, 'words': {}}
    ap = os.path.join(pages_dir, 'atlas.qva')
    if os.path.exists(ap): emb['atlas'] = base64.b64encode(open(ap, 'rb').read()).decode()
    for n in pages:
        k = f'{n:03d}'
        emb['pages'][k] = base64.b64encode(open(os.path.join(pages_dir, k + '.qvp'), 'rb').read()).decode()
        emb['words'][k] = json.load(open(os.path.join(pages_dir, k + '.words.json')))
    payload = json.dumps(emb, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
    html = frag + '\n<script>window.QVP_EMBED=' + payload + ';</script>\n<script>' + qvp + '</script>\n<script>' + app + '</script>\n'
    os.makedirs(dist, exist_ok=True)
    out = os.path.join(dist, 'demo.html')
    open(out, 'w').write(html)
    print(f'{out}: {os.path.getsize(out)/1e6:.2f} MB, {len(pages)} pages embedded')
