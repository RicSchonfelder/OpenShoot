// Smoke test multi-plataforma do addon nativo (sem Electron).
// Uso: npm run build:core && node scripts/smoke-core.mjs
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { mkdirSync, copyFileSync, rmSync, existsSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const arch = process.arch === 'arm64' ? 'arm64' : process.arch
const addonPath = join('core', `openshoot_core.${process.platform}.${arch}.node`)
if (!existsSync(addonPath)) {
  console.error(`[smoke] addon não encontrado: ${addonPath} — rode npm run build:core`)
  process.exit(1)
}
const core = require(join('..', addonPath))

const fixture = join('core', 'fixtures', 'test.jpg')
if (!existsSync(fixture)) {
  console.error('[smoke] fixture core/fixtures/test.jpg ausente')
  process.exit(1)
}

console.log('version:', core.coreVersion())
console.log('hello:', core.hello(process.platform))
console.log('add:', core.add(2, 3))

const dataDir = join(tmpdir(), `openshoot-smoke-${Date.now()}`)
mkdirSync(join(dataDir, 'fotos'), { recursive: true })
copyFileSync(fixture, join(dataDir, 'fotos', 'IMG_0001.jpg'))
core.setup(dataDir)

const scan = core.scanFolder(join(dataDir, 'fotos'), null)
if (scan.added !== 1) throw new Error(`scan inesperado: ${JSON.stringify(scan)}`)
if (core.photoCount() !== 1) throw new Error('catálogo com contagem errada')

const ok = core.deletePhoto(1)
if (!ok || existsSync(join(dataDir, 'fotos', 'IMG_0001.jpg'))) {
  throw new Error('deletePhoto não moveu para a lixeira do sistema')
}

rmSync(dataDir, { recursive: true, force: true })
console.log('[smoke] OK — addon carrega, cataloga e envia p/ lixeira nesta plataforma')
