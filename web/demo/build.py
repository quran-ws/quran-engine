#!/usr/bin/env python3
"""Build the Arabic showcase.

  python3 web/demo/build.py
      dist/demo/: index.html, demo.js, en.js, qvp.js, qvp_ffi.wasm, a link to the pages and a link
      to the letter layers. Serve it: (cd dist/demo && python3 -m http.server 8771), then
      open http://localhost:8771/

Inputs: web/qvp.js, web/qvp_ffi.wasm (cargo build for wasm32, then copy), the pages in
dist/pages/ and the letter layers in dist/letters/ (docs/DATA.md). QVP_PAGES_DIR and
QVP_LETTERS_DIR name other directories, and QVP_ORNAMENTS names an ornament set (.qvo) for
the dress scene, and QVP_TAJWID a file of tajwid rulings (uthmani-hafs.json of
quran-tajweed) in the place of the published package. QVP_FONT_QURAN names a .woff2 file: the typeface for a word of the Quran
outside the page. The page falls back to a web font without it. QVP_FONT_QPC names a directory of QPC page fonts, which the
comparison measures. A layer attaches only to the page file it was built
with, so take the pages and the layers from the same build. The tajwid and recitation
scenes need the layers.

web/demo/config.local.js is optional and git-ignored: it holds the keys of this machine
(web/demo/README.md). The build copies it when it is there. QVP_DEMO_CONFIG names another
config file, and QVP_DEMO_OUT another output directory: a build to share has its own key,
because a key of Quran Lab works on the origins it names and on no other.
"""
import hashlib, json, os, shutil

root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
web = os.path.join(root, 'web'); demo = os.path.join(web, 'demo')
dist = os.path.join(root, 'dist')
out = os.path.abspath(os.environ.get('QVP_DEMO_OUT') or os.path.join(dist, 'demo'))
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
en = open(os.path.join(demo, 'en.js')).read()
# The scripts carry a digest of this build in their address, so a browser that kept an older
# script asks for the new one.
build = hashlib.sha256((html + qvp + app + en).encode()).hexdigest()[:10]
for name in ('qvp.js', 'demo.js', 'en.js'):
    html = html.replace(f'src="{name}"', f'src="{name}?v={build}"')
open(os.path.join(out, 'index.html'), 'w').write(html)
for name, src in (('qvp.js', web), ('demo.js', demo), ('en.js', demo), ('qvp_ffi.wasm', web)):
    shutil.copy(os.path.join(src, name), os.path.join(out, name))
local = os.path.abspath(os.environ.get('QVP_DEMO_CONFIG') or os.path.join(demo, 'config.local.js'))
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
# QVP_TAJWID names a file of tajwid rulings to use in the place of the published package
tj = os.path.join(out, 'tajwid.json')
if os.path.lexists(tj):
    os.remove(tj)
if os.environ.get('QVP_TAJWID'):
    shutil.copy(os.path.expanduser(os.environ['QVP_TAJWID']), tj)
# The font of this machine for a word of the Quran. The repository does not hold it.
fonts = os.path.join(out, 'fonts')
shutil.rmtree(fonts, ignore_errors=True)
for var, name in (('QVP_FONT_QURAN', 'quran.woff2'),):
    if os.environ.get(var):
        os.makedirs(fonts, exist_ok=True)
        shutil.copy(os.path.expanduser(os.environ[var]), os.path.join(fonts, name))
# QVP_FONT_QPC names a directory of QPC page fonts (p3.woff2 to p52.woff2), for the
# comparison's measure. A page font has one glyph for each word of its page, in the private
# use area. The list of those code points goes beside each font, and it needs fontTools.
if os.environ.get('QVP_FONT_QPC'):
    try:
        from fontTools.ttLib import TTFont
        os.makedirs(fonts, exist_ok=True)
        for n in range(3, 53):
            src = os.path.join(os.path.expanduser(os.environ['QVP_FONT_QPC']), f'p{n}.woff2')
            if not os.path.exists(src):
                break
            codes = sorted(c for c in TTFont(src).getBestCmap() if 0xE000 <= c <= 0xF8FF or c >= 0xFB50)
            shutil.copy(src, os.path.join(fonts, f'qpc-{n:03d}.woff2'))
            json.dump(codes, open(os.path.join(fonts, f'qpc-{n:03d}.json'), 'w'))
    except ImportError:
        print('skipped  QVP_FONT_QPC: fontTools is not installed (pip install fonttools brotli)')
link(pages_dir, os.path.join(out, 'pages'))
link(letters_dir, os.path.join(out, 'letters'))
# the page SVGs, for the comparison's measure: the engine's own source (scripts/sync-test-data.sh)
svg_dir = os.path.abspath(os.environ.get('QVP_SVG_DIR') or os.path.join(root, 'pages'))
link(svg_dir if os.path.exists(os.path.join(svg_dir, '003.svg')) else '', os.path.join(out, 'svg'))
print(f'{os.path.relpath(out, root)}/ written; serve it: (cd {os.path.relpath(out, root)} && python3 -m http.server 8771), open http://localhost:8771/')
