# The showcase as a site: nginx with the built page, the engine and the page data.
#
# The image has no build stage. .github/workflows/showcase.yml builds the page into
# web/demo/site/public first (the wasm engine, 604 pages, their letter layers), and this
# file copies the result. The shape matches the static site of quran.ws-server
# (docs/site-templates/static): nginx on port 8080, /healthz, a read-only root filesystem.
FROM nginx:1.27-alpine

COPY public /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD wget -qO- http://127.0.0.1:8080/healthz >/dev/null || exit 1
