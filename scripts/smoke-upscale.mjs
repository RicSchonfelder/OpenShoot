// Smoke test do upscale (ref. Upscayl) — valida o caminho napi→Rust sem
// precisar do modelo ONNX (exercita o fallback bicúbico) e, se o modelo
// 4x-UltraSharp.onnx estiver em core/models, valida a inferência real.
// Uso: npm run build:core && node scripts/smoke-upscale.mjs
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { mkdirSync, copyFileSync, existsSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const arch = process.arch === 'arm64' ? 'arm64' : process.arch
const addonPath = join('core', `openshoot_core.${process.platform}.${arch}.node`)
if (!existsSync(addonPath)) {
  console.error(`[smoke] addon ausente: ${addonPath} — rode npm run build:core`)
  process.exit(1)
}
const core = require(join('..', addonPath))

const fixture = join('core', 'fixtures', 'test.jpg')
if (!existsSync(fixture)) {
  console.error('[smoke] fixture core/fixtures/test.jpg ausente')
  process.exit(1)
}

const dataDir = join(tmpdir(), `openshoot-upscale-${Date.now()}`)
mkdirSync(join(dataDir, 'fotos'), { recursive: true })
copyFileSync(fixture, join(dataDir, 'fotos', 'IMG_0001.jpg'))
core.setup(dataDir)
const scan = core.scanFolder(join(dataDir, 'fotos'), null)
if (scan.added !== 1) throw new Error(`scan inesperado: ${JSON.stringify(scan)}`)

// 1) disponibilidade do modelo real (false se não baixado).
const hasModel = core.upscaleAvailable('4x-UltraSharp')
console.log('upscaleAvailable(4x-UltraSharp):', hasModel)

// 2) fallback bicúbico via napi (sem modelo): deve retornar base64.
const preview = await core.upscalePhoto(1, 'modelo-inexistente', 4, 256)
if (!preview || !preview.startsWith('data:image/jpeg;base64,')) {
  throw new Error('upscalePhoto fallback não retornou preview base64')
}
console.log(`upscalePhoto fallback: OK (${preview.length} chars base64)`)

// 3) export em lote com progresso (fallback) — callback via evento interno.
let last = null
const res = await core.exportUpscaled(
  [1],
  join(dataDir, 'out'),
  'modelo-inexistente',
  2,
  'png',
  90,
  (p) => {
    last = p
  }
)
const obj = JSON.parse(res)
if (!obj.ok || obj.exported !== 1) throw new Error(`exportUp scaled: ${res}`)
if (!existsSync(join(dataDir, 'out', 'IMG_0001.png'))) {
  throw new Error('arquivo upscaled não gravado')
}
console.log('exportUp scaled fallback: OK; progresso final:', JSON.stringify(last), '; files:', obj.files)

if (hasModel) {
  // 4) caminho real (modelo presente): inferência ESRGAN 4x.
  const real = await core.upscalePhoto(1, '4x-UltraSharp', 4, 256)
  if (!real || !real.startsWith('data:image/jpeg;base64,')) {
    throw new Error('upscalePhoto com modelo não retornou preview')
  }
  console.log('upscalePhoto (modelo real): OK')
}

console.log('SMOKE UPSCALE: PASS')
