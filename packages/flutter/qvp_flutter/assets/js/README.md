# assets/js

`qvp.js` lands here: a copy of `web/qvp.js`, the reference JavaScript wrapper, which the web
backend injects before it loads the wasm. The repository does not track the copy.
`scripts/sync-flutter-web-engine.sh` makes it, and the release workflow puts the copy from
the release tag in the pub.dev package.
