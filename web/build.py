#!/usr/bin/env python3
"""Build the demo.

  python3 web/build.py dev        → dist/demo-dev.html   (loads wasm/pages from web/ via fetch; serve web/)
  python3 web/build.py embed 1-21,440-445,582,604 → dist/demo.html (single file, everything inlined; artifact-ready fragment)
"""
import base64, json, os, sys
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
web = os.path.join(root, 'web'); dist = os.path.join(root, 'dist'); os.makedirs(dist, exist_ok=True)
frag = open(os.path.join(web, 'app.html')).read()
qvp = open(os.path.join(web, 'qvp.js')).read()
app = open(os.path.join(web, 'app.js')).read()
mode = sys.argv[1] if len(sys.argv) > 1 else 'dev'

def pages_arg(s):
    out = []
    for part in s.split(','):
        a, _, b = part.partition('-')
        out.extend(range(int(a), int(b or a) + 1))
    return out

if mode == 'dev':
    html = '<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">' + frag + '<script src="qvp.js"></script><script src="app.js"></script></html>'
    open(os.path.join(web, 'index.html'), 'w').write(html)
    print('web/index.html written (serve the web/ directory)')
else:
    pages = pages_arg(sys.argv[2] if len(sys.argv) > 2 else '1-21,440-445,582,604')
    wasm = base64.b64encode(open(os.path.join(web, 'qvp_ffi.wasm'), 'rb').read()).decode()
    emb = {'wasm': wasm, 'pages': {}, 'words': {}}
    for n in pages:
        k = f'{n:03d}'
        emb['pages'][k] = base64.b64encode(open(os.path.join(web, 'pages', k + '.qvp'), 'rb').read()).decode()
        emb['words'][k] = json.load(open(os.path.join(web, 'pages', k + '.words.json')))
    payload = json.dumps(emb, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
    html = frag + '\n<script>window.QVP_EMBED=' + payload + ';</script>\n<script>' + qvp + '</script>\n<script>' + app + '</script>\n'
    out = os.path.join(dist, 'demo.html')
    open(out, 'w').write(html)
    print(f'{out}: {os.path.getsize(out)/1e6:.2f} MB, {len(pages)} pages embedded')
