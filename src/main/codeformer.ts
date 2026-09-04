// Integração CodeFormer local (opt-in, OFF por padrão).
//
// Ponte CLI documentada em docs/CODEFORMER.md: o OpenShoot NÃO distribui pesos
// nem código upstream — o usuário fornece um executável (caminho absoluto) que
// obedece ao contrato. O subprocesso é criado sem shell, sem rede (o app não
// faz nenhuma chamada de rede e impede downloads do hub por env) e escreve
// somente em um diretório de job temporário; os originais nunca são tocados.
import { spawn } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import {
  accessSync,
  constants as fsConstants,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync
} from 'node:fs'
import { isAbsolute, join } from 'node:path'
import type {
  CodeFormerRunResult,
  CodeFormerSettings,
  CodeFormerStatus
} from '../types/codeformer'

export const CODEFORMER_SETTINGS_FILE = 'codeformer-settings.json'
const CODEFORMER_DOC_HINT = 'Consulte docs/CODEFORMER.md para o passo a passo.'

export const DEFAULT_CODEFORMER_SETTINGS: CodeFormerSettings = {
  enabled: false,
  command: null,
  extraArgs: [],
  weightsDir: null,
  fidelityWeight: 0.7,
  timeoutMs: 900_000
}

const MIN_TIMEOUT_MS = 10_000
const MAX_TIMEOUT_MS = 3_600_000
const DEFAULT_FIDELITY_WEIGHT = 0.7

// ---- Settings (parse estrito, sem any) ----

function asStringOrNull(value: unknown): string | null {
  return typeof value === 'string' && value.trim() ? value.trim() : null
}

export function clampFidelityWeight(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) return DEFAULT_FIDELITY_WEIGHT
  return Math.min(1, Math.max(0, value))
}

export function parseCodeFormerSettings(raw: unknown): CodeFormerSettings {
  if (typeof raw !== 'object' || raw === null) return { ...DEFAULT_CODEFORMER_SETTINGS }
  const record = raw as Record<string, unknown>
  const enabled = typeof record.enabled === 'boolean' ? record.enabled : false
  const extraArgs = Array.isArray(record.extraArgs)
    ? record.extraArgs.filter((item): item is string => typeof item === 'string')
    : []
  const timeoutRaw = record.timeoutMs
  const timeoutMs =
    typeof timeoutRaw === 'number' && Number.isInteger(timeoutRaw)
      ? Math.min(MAX_TIMEOUT_MS, Math.max(MIN_TIMEOUT_MS, timeoutRaw))
      : DEFAULT_CODEFORMER_SETTINGS.timeoutMs
  return {
    enabled,
    command: asStringOrNull(record.command),
    extraArgs,
    weightsDir: asStringOrNull(record.weightsDir),
    fidelityWeight: clampFidelityWeight(record.fidelityWeight),
    timeoutMs
  }
}

// ---- Resolução de caminhos (compatível com OPENSHOOT_MODELS_DIR) ----

export function resolveCodeFormerWeightsDir(
  settings: CodeFormerSettings,
  env: NodeJS.ProcessEnv
): string | null {
  if (settings.weightsDir) return settings.weightsDir
  const envWeights = env.OPENSHOOT_CODEFORMER_WEIGHTS_DIR?.trim()
  if (envWeights) return envWeights
  const modelsDir = env.OPENSHOOT_MODELS_DIR?.trim()
  if (modelsDir) return join(modelsDir, 'codeformer')
  return null
}

export function resolveCodeFormerCommand(
  settings: CodeFormerSettings,
  env: NodeJS.ProcessEnv
): string | null {
  return settings.command ?? env.OPENSHOOT_CODEFORMER_COMMAND?.trim() ?? null
}

export function codeFormerCommandExists(commandPath: string): boolean {
  try {
    if (!statSync(commandPath).isFile()) return false
    if (process.platform !== 'win32') accessSync(commandPath, fsConstants.X_OK)
    return true
  } catch {
    return false
  }
}

const WEIGHT_EXTENSIONS = /\.(pth|onnx|pt)$/i

/** Arquivos de pesos no diretório (nível 1 e subdiretórios imediatos, sem ocultos). */
export function findCodeFormerWeightFiles(dir: string): string[] {
  const found: string[] = []
  let entries: string[]
  try {
    entries = readdirSync(dir)
  } catch {
    return found
  }
  for (const name of entries) {
    if (name.startsWith('.')) continue
    const full = join(dir, name)
    let isFile = false
    try {
      isFile = statSync(full).isFile()
    } catch {
      continue
    }
    if (isFile && WEIGHT_EXTENSIONS.test(name)) found.push(name)
  }
  if (found.length) return found
  for (const name of entries) {
    if (name.startsWith('.')) continue
    const sub = join(dir, name)
    try {
      if (!statSync(sub).isDirectory()) continue
    } catch {
      continue
    }
    if (findCodeFormerWeightFilesShallow(sub).length) return [name]
  }
  return found
}

function findCodeFormerWeightFilesShallow(dir: string): string[] {
  try {
    return readdirSync(dir).filter((name) => !name.startsWith('.') && WEIGHT_EXTENSIONS.test(name))
  } catch {
    return []
  }
}

// ---- Status explícito e acionável ----

export function getCodeFormerStatus(
  settings: CodeFormerSettings,
  env: NodeJS.ProcessEnv
): CodeFormerStatus {
  if (!settings.enabled) {
    return {
      enabled: false,
      level: 'disabled',
      commandPath: null,
      commandFound: false,
      weightsDir: null,
      weightsFound: false,
      errors: [],
      hints: ['Recurso opt-in e desligado por padrão. Ative na bancada de restauração se quiser usar.']
    }
  }
  const errors: string[] = []
  const hints: string[] = []
  const commandPath = resolveCodeFormerCommand(settings, env)
  const commandFound = commandPath !== null && codeFormerCommandExists(commandPath)
  if (commandPath === null) {
    errors.push('Comando do CodeFormer não configurado.')
    hints.push('Defina o caminho absoluto do executável da ponte CLI nas configurações ou a variável OPENSHOOT_CODEFORMER_COMMAND.')
    hints.push(CODEFORMER_DOC_HINT)
  } else if (!commandFound) {
    errors.push(`Comando não encontrado ou não executável: ${commandPath}`)
    hints.push('Verifique o caminho absoluto e a permissão de execução (chmod +x em Linux/macOS).')
    hints.push(CODEFORMER_DOC_HINT)
  }
  const weightsDir = resolveCodeFormerWeightsDir(settings, env)
  const weightsFound = weightsDir !== null && findCodeFormerWeightFiles(weightsDir).length > 0
  if (weightsDir === null) {
    errors.push('Diretório de pesos não definido.')
    hints.push('Defina weightsDir nas configurações, OPENSHOOT_CODEFORMER_WEIGHTS_DIR ou coloque os pesos em "$OPENSHOOT_MODELS_DIR/codeformer".')
    hints.push(CODEFORMER_DOC_HINT)
  } else if (!weightsFound) {
    errors.push(`Nenhum arquivo de pesos (.pth/.onnx/.pt) encontrado em: ${weightsDir}`)
    hints.push('Baixe os pesos oficiais do upstream manualmente e coloque-os nesse diretório (ex.: codeformer-v0.1.0.pth e o detector de faces do facexlib). O OpenShoot não baixa pesos.')
    hints.push(CODEFORMER_DOC_HINT)
  }
  const ready = commandFound && weightsFound
  return {
    enabled: true,
    level: ready ? 'ready' : 'error',
    commandPath,
    commandFound,
    weightsDir,
    weightsFound,
    errors,
    hints
  }
}

// ---- Contrato CLI v1 ----

export interface CodeFormerContractArgs {
  inputPath: string
  outputDir: string
  fidelityWeight: number
  weightsDir: string | null
}

export function buildCodeFormerArgs(opts: CodeFormerContractArgs): string[] {
  const args = [
    '--input',
    opts.inputPath,
    '--output-dir',
    opts.outputDir,
    '--fidelity-weight',
    clampFidelityWeight(opts.fidelityWeight).toFixed(2)
  ]
  if (opts.weightsDir) args.push('--weights-dir', opts.weightsDir)
  return args
}

/** Exatamente um arquivo de imagem (.png/.jpg/.jpeg) não oculto; ignora logs. */
export function pickOutputFileName(names: readonly string[]): string | null {
  const images = names.filter((name) => !name.startsWith('.') && /\.(png|jpe?g)$/i.test(name))
  return images.length === 1 ? images[0] : null
}

export function detectImageMime(bytes: Uint8Array): 'image/png' | 'image/jpeg' | null {
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return 'image/jpeg'
  }
  const pngMagic = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
  if (bytes.length >= 8 && pngMagic.every((byte, index) => bytes[index] === byte)) {
    return 'image/png'
  }
  return null
}

// ---- Runner (subprocesso sem shell, sem rede, saída validada) ----

const STDERR_TAIL_LIMIT = 4000

function tail(text: string, limit: number): string {
  const trimmed = text.trim()
  return trimmed.length <= limit ? trimmed : `…${trimmed.slice(-limit)}`
}

export interface CodeFormerRunOptions {
  settings: CodeFormerSettings
  env: NodeJS.ProcessEnv
  sourcePath: string
  /** Diretório-pai onde o job temporário é criado (ex.: userData/codeformer-jobs). */
  jobsRoot: string
}

interface SpawnOutcome {
  code: number | null
  timedOut: boolean
  spawnError: string | null
  stderrTail: string
}

function spawnCodeFormer(
  command: string,
  args: readonly string[],
  opts: { cwd: string; timeoutMs: number; env: NodeJS.ProcessEnv }
): Promise<SpawnOutcome> {
  return new Promise((resolveOutcome) => {
    // Sem shell (lista de argv), stdio apenas com stderr coletado, sem rede pelo app.
    const child = spawn(command, [...args], {
      shell: false,
      cwd: opts.cwd,
      killSignal: 'SIGKILL',
      windowsHide: true,
      stdio: ['ignore', 'ignore', 'pipe'],
      // Impede downloads por hub nas pontes baseadas no upstream; nada é buscado pelo app.
      env: { ...opts.env, HF_HUB_OFFLINE: '1', TRANSFORMERS_OFFLINE: '1' }
    })
    let stderrTail = ''
    let timedOut = false
    let spawnError: string | null = null
    let settled = false
    // Timeout manual: emite 'close' com sinal (sem evento 'error') no Node atual.
    const timer = setTimeout(() => {
      timedOut = true
      try {
        child.kill('SIGKILL')
      } catch {
        /* processo já terminou */
      }
    }, opts.timeoutMs)
    const finish = (code: number | null) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      resolveOutcome({ code, timedOut, spawnError, stderrTail })
    }
    child.stderr?.on('data', (chunk: Buffer) => {
      stderrTail = tail(`${stderrTail}${chunk.toString('utf8')}`, STDERR_TAIL_LIMIT)
    })
    child.on('error', (error: NodeJS.ErrnoException) => {
      spawnError = error.message
      // Falha de spawn pode não emitir 'close' — resolver aqui também.
      finish(null)
    })
    child.on('close', (code) => {
      finish(code)
    })
  })
}

function failure(error: string, hint?: string): CodeFormerRunResult {
  return { ok: false, error, hint }
}

export async function runCodeFormerRestore(opts: CodeFormerRunOptions): Promise<CodeFormerRunResult> {
  const { settings, env, sourcePath, jobsRoot } = opts
  const status = getCodeFormerStatus(settings, env)
  if (status.level !== 'ready') {
    const hint = status.hints[0]
    return failure(status.errors[0] ?? 'CodeFormer não está pronto.', hint)
  }
  if (!isAbsolute(sourcePath)) return failure('Caminho da foto deve ser absoluto.')
  try {
    if (!statSync(sourcePath).isFile()) return failure('Foto de origem não encontrada.')
  } catch {
    return failure('Foto de origem não encontrada.')
  }
  const command = resolveCodeFormerCommand(settings, env)
  if (command === null) return failure('Comando do CodeFormer não configurado.')

  const jobDir = join(jobsRoot, randomUUID())
  try {
    mkdirSync(jobsRoot, { recursive: true, mode: 0o700 })
    mkdirSync(jobDir, { recursive: true, mode: 0o700 })
  } catch (error) {
    return failure(`Falha ao criar diretório temporário do job: ${String(error)}`)
  }

  try {
    const argv = [
      ...settings.extraArgs,
      ...buildCodeFormerArgs({
        inputPath: sourcePath,
        outputDir: jobDir,
        fidelityWeight: settings.fidelityWeight,
        weightsDir: status.weightsDir
      })
    ]
    const outcome = await spawnCodeFormer(command, argv, {
      cwd: jobDir,
      timeoutMs: settings.timeoutMs,
      env
    })
    if (outcome.timedOut) {
      return failure(
        `CodeFormer excedeu o tempo limite (${Math.round(settings.timeoutMs / 1000)} s) e foi encerrado.`,
        'Aumente timeoutMs nas configurações ou teste com uma imagem menor.'
      )
    }
    if (outcome.spawnError !== null) {
      return failure(`Falha ao iniciar o CodeFormer: ${outcome.spawnError}`, CODEFORMER_DOC_HINT)
    }
    if (outcome.code !== 0) {
      const detail = outcome.stderrTail ? ` Saída do comando: ${outcome.stderrTail}` : ''
      return failure(
        `CodeFormer terminou com código ${outcome.code ?? 'desconhecido'}.${detail}`,
        'Verifique dependências do ambiente da ponte CLI (Python/PyTorch) e os pesos. ' + CODEFORMER_DOC_HINT
      )
    }
    let names: string[]
    try {
      names = readdirSync(jobDir)
    } catch (error) {
      return failure(`Falha ao ler a saída do job: ${String(error)}`)
    }
    const outputName = pickOutputFileName(names)
    if (outputName === null) {
      return failure(
        'A saída não contém exatamente um arquivo de imagem (.png/.jpg/.jpeg).',
        'A ponte CLI deve gravar um único resultado direto em --output-dir. ' + CODEFORMER_DOC_HINT
      )
    }
    const outputPath = join(jobDir, outputName)
    let bytes: Buffer
    try {
      bytes = readFileSync(outputPath)
    } catch (error) {
      return failure(`Falha ao ler o arquivo de saída: ${String(error)}`)
    }
    if (bytes.length === 0) return failure('Arquivo de saída vazio.')
    const mime = detectImageMime(bytes)
    if (mime === null) {
      return failure('Arquivo de saída não é um JPEG/PNG válido.')
    }
    return {
      ok: true,
      dataUrl: `data:${mime};base64,${bytes.toString('base64')}`,
      mime,
      bytes: bytes.length,
      outputName
    }
  } finally {
    // A cópia temporária nunca sobra: job dir é removido após ler/validar.
    try {
      rmSync(jobDir, { recursive: true, force: true })
    } catch {
      /* melhor esforço */
    }
  }
}
