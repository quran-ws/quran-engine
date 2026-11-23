#!/usr/bin/env python3
"""Build the Arabic showcase.

  python3 web/demo/build.py
      dist/demo/: index.html, demo.js, qvp.js, qvp_ffi.wasm, a link to the pages and a link
      to the letter layers. Serve it: (cd dist/demo && python3 -m http.server 8771), then
      open http://localhost:8771/

Inputs: web/qvp.js, web/qvp_ffi.wasm (cargo build for wasm32, then copy), the pages in
dist/pages/ and the letter layers in dist/letters/ (docs/DATA.md). QVP_PAGES_DIR and
QVP_LETTERS_DIR name other directories, and QVP_ORNAMENTS names an ornament set (.qvo) for
the dress scene. A layer attaches only to the page file it was built
with, so take the pages and the layers from the same build. The tajwid and recitation
scenes need the layers.

web/demo/config.local.js is optional and git-ignored: it holds the keys of this machine
(web/demo/README.md). The build copies it when it is there.
"""
import hashlib, os, shutil

root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
web = os.path.join(root, 'web'); demo = os.path.join(web, 'demo')
dist = os.path.join(root, 'dist'); out = os.path.join(dist, 'demo')
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


os.makedirs(out, exist_ok=True)
html = open(os.path.join(demo, 'index.html')).read()
qvp = open(os.path.join(web, 'qvp.js')).read()
app = open(os.path.join(demo, 'demo.js')).read()
# The scripts carry a digest of this build in their address, so a browser that kept an older
# script asks for the new one.
build = hashlib.sha256((html + qvp + app).encode()).hexdigest()[:10]
html = html.replace('src="qvp.js"', f'src="qvp.js?v={build}"').replace('src="demo.js"', f'src="demo.js?v={build}"')
open(os.path.join(out, 'index.html'), 'w').write(html)
for name, src in (('qvp.js', web), ('demo.js', demo), ('qvp_ffi.wasm', web)):
    shutil.copy(os.path.join(src, name), os.path.join(out, name))
local = os.path.join(demo, 'config.local.js')
if os.path.exists(local):
    shutil.copy(local, os.path.join(out, 'config.local.js'))
elif os.path.exists(os.path.join(out, 'config.local.js')):
    os.remove(os.path.join(out, 'config.local.js'))
# QVP_ORNAMENTS names an ornament set to use in the place of the one beside the pages
orn = os.path.join(out, 'ornaments.qvo')
if os.path.lexists(orn):
    os.remove(orn)
if os.environ.get('QVP_ORNAMENTS'):
    os.symlink(os.path.abspath(os.environ['QVP_ORNAMENTS']), orn)
link(pages_dir, os.path.join(out, 'pages'))
link(letters_dir, os.path.join(out, 'letters'))
print('dist/demo/ written; serve it: (cd dist/demo && python3 -m http.server 8771), open http://localhost:8771/')
