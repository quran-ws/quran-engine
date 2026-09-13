import { createServer } from 'node:http'
import { readFile, mkdir, writeFile } from 'node:fs/promises'
import { resolve, extname, sep } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createHash } from 'node:crypto'

const root = fileURLToPath(new URL('.', import.meta.url))
const types = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.bin': 'application/octet-stream' }

createServer(async (request, response) => {
  response.setHeader('Cross-Origin-Opener-Policy', 'same-origin')
  response.setHeader('Cross-Origin-Embedder-Policy', 'require-corp')
  try {
    if (request.url === '/provenance') {
      const manifest = JSON.parse(await readFile(resolve(root, 'data/manifest.json'), 'utf8'))
      const files = ['index.html', 'app.js', 'renderers.js', 'worker.js', 'geometry.js', 'export.c', 'export.sh', 'server.mjs', 'pnpm-lock.yaml', 'data/manifest.json']
      for (const number of manifest.pages) {
        const page = String(number).padStart(3, '0')
        files.push(`data/${page}.bin`)
      }
      const hashes = {}
      for (const filename of files) hashes[filename] = createHash('sha256').update(await readFile(resolve(root, filename))).digest('hex')
      response.writeHead(200, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' })
      response.end(JSON.stringify({ manifest, sha256: hashes }))
      return
    }
    if (request.method === 'POST' && request.url === '/results') {
      let body = ''
      for await (const chunk of request) {
        body += chunk
        if (body.length > 15_000_000) throw new Error('Result too large')
      }
      const result = JSON.parse(body)
      await mkdir(resolve(root, 'results'), { recursive: true })
      const filename = `${new Date().toISOString().replaceAll(':', '-')}.json`
      await writeFile(resolve(root, 'results', filename), JSON.stringify(result, null, 2))
      response.writeHead(200, { 'Content-Type': 'application/json' })
      response.end(JSON.stringify({ file: `results/${filename}` }))
      return
    }
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname)
    const filename = resolve(root, `.${pathname === '/' ? '/index.html' : pathname}`)
    if (!filename.startsWith(root.endsWith(sep) ? root : root + sep)) throw new Error('Invalid path')
    const data = await readFile(filename)
    response.writeHead(200, { 'Content-Type': types[extname(filename)] || 'application/octet-stream', 'Cache-Control': 'no-store' })
    response.end(data)
  } catch (error) {
    response.writeHead(404, { 'Content-Type': 'text/plain' })
    response.end(error.message)
  }
}).listen(Number(process.env.PORT || 4173), process.env.HOST || '127.0.0.1', () => {
  console.log(`QVP benchmark: http://${process.env.HOST || '127.0.0.1'}:${process.env.PORT || 4173}`)
})
