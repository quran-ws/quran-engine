# Security

The engine parses binary page files (`.qvp`, `.qva`) that an app may have downloaded, and it
runs inside iOS, Android, Flutter, React Native and browser hosts through a C ABI and wasm.
A malformed file must never crash or compromise the host. If you find a way to make it,
we want to know.

## Reporting

Email security@quran.ws, or open a private security advisory on GitHub
(Security → Advisories → Report a vulnerability). Do not open a public issue.

Include the file or input that triggers it and the platform. You will get an acknowledgement
within three days and a fix or a timeline within fourteen.

## Scope

- Decoding in `crates/qvp-format` and every `qvp_*` entry point in `crates/qvp-ffi`.
- The wrappers' marshalling of untrusted bytes.
- The published packages and the page data release (integrity of the tarball and CDN).

Out of scope: the demo apps' UI, and denial of service by feeding the engine very large
legitimate files.

## Supported versions

The latest minor release receives fixes. Pre-1.0, that is the latest release.
