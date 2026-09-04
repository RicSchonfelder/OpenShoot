// Testes determinísticos da integração CodeFormer (sem pesos, sem GPU, sem rede).
// A ponte CLI é simulada por scripts Node locais; nada do upstream é necessário.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { chmodSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  DEFAULT_CODEFORMER_SETTINGS,
  buildCodeFormerArgs,
  clampFidelityWeight,
  codeFormerCommandExists,
  detectImageMime,
  findCodeFormerWeightFiles,
  getCodeFormerStatus,
  parseCodeFormerSettings,
  pickOutputFileName,
  resolveCodeFormerCommand,
  resolveCodeFormerWeightsDir,
  runCodeFormerRestore
} from '../../src/main/codeformer'
import type { CodeFormerSettings } from '../../src/types/codeformer'

const FAKE_PNG_BASE64 = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg=='

function pngBytes(): Buffer {
  return Buffer.from(FAKE_PNG_BASE64, 'base64')
}

function makeTempRoot(prefix: string): string {
  return mkdtempSync(join(tmpdir(), `openshoot-cf-${prefix}-`))
}

function settingsFor(overrides: Partial<CodeFormerSettings>): CodeFormerSettings {
  return { ...DEFAULT_CODEFORMER_SETTINGS, ...overrides }
}

function writeScript(dir: string, name: string, body: string): string {
  const path = join(dir, name)
  writeFileSync(path, body, 'utf8')
  chmodSync(path, 0o755)
  return path
}

let bridgeCounter = 0

/** Ambiente "pronto": pesos fake + ponte CLI simulada por Node. */
function readySetup(root: string, bridgeBody: string): { settings: CodeFormerSettings; weights: string } {
  bridgeCounter += 1
  const weights = join(root, 'weights')
  mkdirSync(weights, { recursive: true })
  writeFileSync(join(weights, 'codeformer-v0.1.0.pth'), 'fake', 'utf8')
  const bridge = writeScript(root, `bridge-${bridgeCounter}.cjs`, bridgeBody)
  return {
    settings: settingsFor({
      enabled: true,
      command: process.execPath,
      extraArgs: [bridge],
      weightsDir: weights,
      timeoutMs: 60_000
    }),
    weights
  }
}

const BRIDGE_OK = [
  "const { writeFileSync } = require('node:fs')",
  'const i = process.argv.indexOf("--output-dir")',
  'const out = process.argv[i + 1]',
  `writeFileSync(out + "/restored.png", Buffer.from("${FAKE_PNG_BASE64}", "base64"))`,
  'process.exit(0)'
].join('\n')

test('default settings are opt-in OFF and safe', () => {
  assert.equal(DEFAULT_CODEFORMER_SETTINGS.enabled, false)
  assert.equal(DEFAULT_CODEFORMER_SETTINGS.command, null)
  assert.equal(DEFAULT_CODEFORMER_SETTINGS.weightsDir, null)
  assert.equal(DEFAULT_CODEFORMER_SETTINGS.fidelityWeight, 0.7)
  const parsed = parseCodeFormerSettings(undefined)
  assert.equal(parsed.enabled, false)
})

test('parse ignores invalid fields and clamps weight/timeout', () => {
  const parsed = parseCodeFormerSettings({
    enabled: 'yes',
    command: 42,
    extraArgs: 'nope',
    fidelityWeight: 7,
    timeoutMs: 1
  })
  assert.equal(parsed.enabled, false)
  assert.equal(parsed.command, null)
  assert.deepEqual(parsed.extraArgs, [])
  assert.equal(parsed.fidelityWeight, 1)
  assert.equal(parsed.timeoutMs, 10_000)
  assert.equal(parseCodeFormerSettings({ timeoutMs: 'x' }).timeoutMs, DEFAULT_CODEFORMER_SETTINGS.timeoutMs)
  assert.equal(clampFidelityWeight(-3), 0)
  assert.equal(clampFidelityWeight(0.42), 0.42)
  assert.equal(clampFidelityWeight('x'), 0.7)
})

test('weights dir resolution: settings > env weights > OPENSHOOT_MODELS_DIR/codeformer > null', () => {
  assert.equal(
    resolveCodeFormerWeightsDir(settingsFor({ weightsDir: '/custom/weights' }), { OPENSHOOT_CODEFORMER_WEIGHTS_DIR: '/env/weights' }),
    '/custom/weights'
  )
  assert.equal(
    resolveCodeFormerWeightsDir(settingsFor({ weightsDir: null }), { OPENSHOOT_CODEFORMER_WEIGHTS_DIR: '/env/weights' }),
    '/env/weights'
  )
  assert.equal(
    resolveCodeFormerWeightsDir(settingsFor({}), { OPENSHOOT_MODELS_DIR: '/models' }),
    join('/models', 'codeformer')
  )
  assert.equal(resolveCodeFormerWeightsDir(settingsFor({}), {}), null)
})

test('command resolution: settings wins over env', () => {
  assert.equal(resolveCodeFormerCommand(settingsFor({ command: '/usr/local/bin/bridge' }), { OPENSHOOT_CODEFORMER_COMMAND: '/other' }), '/usr/local/bin/bridge')
  assert.equal(resolveCodeFormerCommand(settingsFor({ command: null }), { OPENSHOOT_CODEFORMER_COMMAND: '/other' }), '/other')
  assert.equal(resolveCodeFormerCommand(settingsFor({ command: null }), {}), null)
})

test('buildCodeFormerArgs follows CLI contract v1', () => {
  const args = buildCodeFormerArgs({
    inputPath: '/fotos/a.png',
    outputDir: '/tmp/job',
    fidelityWeight: 0.7,
    weightsDir: '/weights'
  })
  assert.deepEqual(args, ['--input', '/fotos/a.png', '--output-dir', '/tmp/job', '--fidelity-weight', '0.70', '--weights-dir', '/weights'])
  const withoutWeights = buildCodeFormerArgs({ inputPath: '/a.png', outputDir: '/o', fidelityWeight: 1, weightsDir: null })
  assert.deepEqual(withoutWeights, ['--input', '/a.png', '--output-dir', '/o', '--fidelity-weight', '1.00'])
  for (const arg of args) assert.ok(!/^https?:/i.test(arg), 'contrato não pode conter URLs')
})

test('detectImageMime by magic bytes', () => {
  assert.equal(detectImageMime(pngBytes()), 'image/png')
  assert.equal(detectImageMime(Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0x01])), 'image/jpeg')
  assert.equal(detectImageMime(Buffer.from('not an image')), null)
  assert.equal(detectImageMime(Buffer.from([0x89, 0x50])), null)
})

test('pickOutputFileName requires exactly one image', () => {
  assert.equal(pickOutputFileName(['restored.png']), 'restored.png')
  assert.equal(pickOutputFileName(['.hidden', 'restored.jpg', 'run.log']), 'restored.jpg')
  assert.equal(pickOutputFileName(['a.png', 'b.png']), null)
  assert.equal(pickOutputFileName(['only.log']), null)
  assert.equal(pickOutputFileName([]), null)
})

test('status: disabled by default; enabled without setup gives actionable errors; ready when configured', () => {
  const disabled = getCodeFormerStatus(DEFAULT_CODEFORMER_SETTINGS, {})
  assert.equal(disabled.level, 'disabled')
  assert.equal(disabled.enabled, false)

  const broken = getCodeFormerStatus(settingsFor({ enabled: true }), {})
  assert.equal(broken.level, 'error')
  assert.ok(broken.errors.some((message) => message.includes('Comando')))
  assert.ok(broken.errors.some((message) => message.includes('pesos')))
  assert.ok(broken.hints.length > 0)

  const root = makeTempRoot('status')
  try {
    const weights = join(root, 'weights')
    mkdirSync(weights)
    writeFileSync(join(weights, 'codeformer-v0.1.0.pth'), 'fake', 'utf8')
    const command = writeScript(root, 'bridge', "#!/bin/sh\nexit 0\n")
    const ready = getCodeFormerStatus(settingsFor({ enabled: true, command, weightsDir: weights }), {})
    assert.equal(ready.level, 'ready')
    assert.equal(ready.commandFound, true)
    assert.equal(ready.weightsFound, true)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('commandExists validates file and exec bit (POSIX)', () => {
  const root = makeTempRoot('cmdexists')
  try {
    const file = join(root, 'tool')
    writeFileSync(file, '#!/bin/sh\n', 'utf8')
    if (process.platform === 'win32') {
      assert.equal(codeFormerCommandExists(file), true)
    } else {
      chmodSync(file, 0o644)
      assert.equal(codeFormerCommandExists(file), false)
      chmodSync(file, 0o755)
      assert.equal(codeFormerCommandExists(file), true)
    }
    assert.equal(codeFormerCommandExists(join(root, 'missing')), false)
    assert.equal(codeFormerCommandExists(root), false)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('findCodeFormerWeightFiles scans top level and immediate subdirs', () => {
  const root = makeTempRoot('weights')
  try {
    assert.deepEqual(findCodeFormerWeightFiles(join(root, 'missing')), [])
    const weights = join(root, 'weights')
    mkdirSync(weights)
    assert.deepEqual(findCodeFormerWeightFiles(weights), [])
    writeFileSync(join(weights, 'codeformer-v0.1.0.pth'), 'x', 'utf8')
    writeFileSync(join(weights, 'notes.txt'), 'x', 'utf8')
    assert.deepEqual(findCodeFormerWeightFiles(weights), ['codeformer-v0.1.0.pth'])
    const nested = join(weights, 'sub')
    mkdirSync(nested)
    writeFileSync(join(nested, 'RetinaFace-Resnet50.pth'), 'x', 'utf8')
    assert.deepEqual(findCodeFormerWeightFiles(join(root, 'weights')), ['codeformer-v0.1.0.pth'])
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('run ok: fake bridge produces validated PNG; job dir cleaned; source untouched', async () => {
  const root = makeTempRoot('run-ok')
  try {
    const { settings } = readySetup(root, BRIDGE_OK)
    const source = join(root, 'com espaço.png')
    writeFileSync(source, pngBytes())
    const sourceBefore = readFileSync(source)
    const jobsRoot = join(root, 'jobs')
    const result = await runCodeFormerRestore({ settings, env: {}, sourcePath: source, jobsRoot })
    assert.equal(result.ok, true)
    if (!result.ok) return
    assert.ok(result.dataUrl.startsWith('data:image/png;base64,'))
    assert.ok(result.bytes > 0)
    assert.equal(readdirSync(jobsRoot).length, 0, 'diretório de job deve ser removido')
    assert.deepEqual(readFileSync(source), sourceBefore, 'original jamais modificado')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('run failure: non-zero exit propagates stderr and offline env guard', async () => {
  const root = makeTempRoot('run-fail')
  try {
    const { settings } = readySetup(root, [
      "console.error('dependência ausente: torch HF_HUB_OFFLINE=' + process.env.HF_HUB_OFFLINE)",
      'process.exit(1)'
    ].join('\n'))
    const source = join(root, 'input.png')
    writeFileSync(source, pngBytes())
    const result = await runCodeFormerRestore({ settings, env: {}, sourcePath: source, jobsRoot: join(root, 'jobs') })
    assert.equal(result.ok, false)
    if (result.ok) return
    assert.ok(result.error.includes('código 1'))
    assert.ok(result.error.includes('dependência ausente'))
    assert.ok(result.error.includes('HF_HUB_OFFLINE=1'), 'env offline deve ser propagado ao subprocesso')
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('run validates output contract: two images and invalid bytes fail', async () => {
  const root = makeTempRoot('run-contract')
  try {
    const source = join(root, 'input.png')
    writeFileSync(source, pngBytes())
    const jobsRoot = join(root, 'jobs')

    const two = readySetup(root, [
      "const { writeFileSync } = require('node:fs')",
      'const i = process.argv.indexOf("--output-dir")',
      `writeFileSync(process.argv[i + 1] + "/a.png", Buffer.from("${FAKE_PNG_BASE64}", "base64"))`,
      `writeFileSync(process.argv[i + 1] + "/b.png", Buffer.from("${FAKE_PNG_BASE64}", "base64"))`
    ].join('\n'))
    const twoResult = await runCodeFormerRestore({ settings: two.settings, env: {}, sourcePath: source, jobsRoot })
    assert.equal(twoResult.ok, false)
    if (!twoResult.ok) assert.ok(twoResult.error.includes('exatamente um'))

    const garbage = readySetup(root, [
      "const { writeFileSync } = require('node:fs')",
      'const i = process.argv.indexOf("--output-dir")',
      "writeFileSync(process.argv[i + 1] + '/out.png', 'texto não é imagem')"
    ].join('\n'))
    const garbageResult = await runCodeFormerRestore({ settings: garbage.settings, env: {}, sourcePath: source, jobsRoot })
    assert.equal(garbageResult.ok, false)
    if (!garbageResult.ok) assert.ok(garbageResult.error.includes('JPEG/PNG'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('argv order: extraArgs precede contract args, spaces preserved, no shell splitting', async () => {
  const root = makeTempRoot('argv-order')
  try {
    const bridgeBody = [
      "console.error(JSON.stringify(process.argv.slice(1)))",
      'process.exit(2)'
    ].join('\n')
    const { settings } = readySetup(root, bridgeBody)
    const bridge = settings.extraArgs[0]
    const source = join(root, 'foto 001.png')
    writeFileSync(source, pngBytes())
    const jobsRoot = join(root, 'jobs')
    // Padrão documentado: command pode ser um interpretador; extraArgs começam
    // com o script da ponte, seguido das flags do usuário.
    const withExtra: CodeFormerSettings = { ...settings, extraArgs: [bridge, '--flag-usuario', 'valor com espaço'] }
    const result = await runCodeFormerRestore({ settings: withExtra, env: {}, sourcePath: source, jobsRoot })
    assert.equal(result.ok, false)
    if (result.ok) return
    const jsonMatch = result.error.match(/\[[\s\S]*\]/)
    assert.ok(jsonMatch, 'stderr deve conter o argv em JSON')
    const argv = JSON.parse(jsonMatch[0]) as unknown[]
    assert.equal(argv[0], bridge)
    assert.equal(argv[1], '--flag-usuario')
    assert.equal(argv[2], 'valor com espaço', 'arg com espaço deve chegar intacto (sem shell)')
    assert.equal(argv[3], '--input')
    assert.equal(argv[4], source)
    assert.equal(argv[5], '--output-dir')
    const jobDir = argv[6]
    assert.equal(typeof jobDir, 'string')
    assert.ok((jobDir as string).startsWith(jobsRoot))
    assert.equal(argv[7], '--fidelity-weight')
    assert.equal(argv[8], '0.70')
    assert.equal(argv[9], '--weights-dir')
    assert.equal(argv[10], settings.weightsDir)
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('run fails fast without spawning when disabled or source missing', async () => {
  const root = makeTempRoot('run-fast-fail')
  try {
    const disabled = await runCodeFormerRestore({
      settings: DEFAULT_CODEFORMER_SETTINGS,
      env: {},
      sourcePath: join(root, 'x.png'),
      jobsRoot: join(root, 'jobs')
    })
    assert.equal(disabled.ok, false)
    if (!disabled.ok) assert.ok(disabled.error.length > 0)

    const { settings } = readySetup(root, BRIDGE_OK)
    const missingSource = await runCodeFormerRestore({
      settings,
      env: {},
      sourcePath: join(root, 'inexistente.png'),
      jobsRoot: join(root, 'jobs')
    })
    assert.equal(missingSource.ok, false)
    if (!missingSource.ok) assert.ok(missingSource.error.includes('origem'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})

test('timeout kills a hanging bridge (~10 s)', async () => {
  const root = makeTempRoot('run-timeout')
  try {
    const { settings } = readySetup(root, 'setTimeout(() => process.exit(0), 120000)')
    const source = join(root, 'input.png')
    writeFileSync(source, pngBytes())
    const result = await runCodeFormerRestore({
      settings: { ...settings, timeoutMs: 10_000 },
      env: {},
      sourcePath: source,
      jobsRoot: join(root, 'jobs')
    })
    assert.equal(result.ok, false)
    if (!result.ok) assert.ok(result.error.includes('tempo limite'))
  } finally {
    rmSync(root, { recursive: true, force: true })
  }
})
