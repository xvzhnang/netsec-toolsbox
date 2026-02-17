import fs from 'node:fs/promises'
import path from 'node:path'
import https from 'node:https'
import { fileURLToPath } from 'node:url'
import { build } from 'esbuild'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const rootDir = path.resolve(__dirname, '..')

const sourceDir = path.join(rootDir, 'public', 'javascripts')
const vendorDir = path.join(sourceDir, 'vendor')
const outputDir = path.join(rootDir, 'wiki', 'docs', 'javascripts')
const outputFile = path.join(outputDir, 'bundle.js')

const MATHJAX_URL = 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-svg.js'
const MERMAID_URL = 'https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js'

async function fileExists(filePath) {
  try {
    await fs.access(filePath)
    return true
  } catch {
    return false
  }
}

function download(url, destPath) {
  return new Promise((resolve, reject) => {
    const request = https.get(url, (res) => {
      const status = res.statusCode ?? 0
      const location = res.headers.location
      if (status >= 300 && status < 400 && location) {
        res.resume()
        resolve(download(location, destPath))
        return
      }
      if (status !== 200) {
        res.resume()
        reject(new Error(`下载失败: ${url} (HTTP ${status})`))
        return
      }
      const file = fs
        .open(destPath, 'w')
        .then(async (handle) => {
          try {
            await new Promise((r, e) => {
              res.pipe(handle.createWriteStream())
              res.on('end', r)
              res.on('error', e)
            })
          } finally {
            await handle.close()
          }
        })
      resolve(file)
    })
    request.on('error', reject)
  })
}

async function ensureVendorFiles() {
  await fs.mkdir(vendorDir, { recursive: true })

  const mathjaxFile = path.join(vendorDir, 'tex-svg.js')
  const mermaidFile = path.join(vendorDir, 'mermaid.min.js')

  if (!(await fileExists(mathjaxFile))) {
    await download(MATHJAX_URL, mathjaxFile)
  }
  if (!(await fileExists(mermaidFile))) {
    await download(MERMAID_URL, mermaidFile)
  }

  return { mathjaxFile, mermaidFile }
}

async function main() {
  await fs.mkdir(outputDir, { recursive: true })
  await ensureVendorFiles()

  const entry = [
    `import "./mathjax.js";`,
    `import "./vendor/tex-svg.js";`,
    `import "./vendor/mermaid.min.js";`,
    `import "./mermaid.js";`,
    `import "./init.js";`,
  ].join('\n')

  await build({
    stdin: {
      contents: entry,
      resolveDir: sourceDir,
      sourcefile: 'wiki-bundle.entry.js',
      loader: 'js',
    },
    bundle: true,
    platform: 'browser',
    format: 'iife',
    target: ['es2018'],
    minify: true,
    legalComments: 'none',
    sourcemap: false,
    write: true,
    outfile: outputFile,
  })
}

main().catch((err) => {
  console.error(err)
  process.exit(1)
})
