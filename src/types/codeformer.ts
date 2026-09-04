// CodeFormer local (opt-in): tipos compartilhados entre main, preload e renderer.
// Integração é 100% local/offline via subprocesso sem shell; pesos nunca são
// distribuídos pelo OpenShoot (ver docs/CODEFORMER.md).

export interface CodeFormerSettings {
  /** Opt-in explícito. Padrão: false (desligado). */
  enabled: boolean
  /** Caminho ABSOLUTO do executável/ponte CLI fornecido pelo usuário. */
  command: string | null
  /** Argumentos estáticos extras (sem shell; passados literalmente). */
  extraArgs: string[]
  /** Diretório de pesos; null resolve via env (ver resolveCodeFormerWeightsDir). */
  weightsDir: string | null
  /** Peso de fidelidade 0..1 (0 = mais restauração, 1 = mais fidelidade). */
  fidelityWeight: number
  /** Timeout do subprocesso em ms. */
  timeoutMs: number
}

export type CodeFormerLevel = 'disabled' | 'ready' | 'error'

export interface CodeFormerStatus {
  enabled: boolean
  level: CodeFormerLevel
  commandPath: string | null
  commandFound: boolean
  weightsDir: string | null
  weightsFound: boolean
  /** Mensagens acionáveis (pt-BR) quando level !== 'ready'. */
  errors: string[]
  /** Próximos passos sugeridos (pt-BR). */
  hints: string[]
}

export interface CodeFormerRunOk {
  ok: true
  dataUrl: string
  mime: 'image/png' | 'image/jpeg'
  bytes: number
  outputName: string
}

export interface CodeFormerRunFail {
  ok: false
  error: string
  hint?: string
}

export type CodeFormerRunResult = CodeFormerRunOk | CodeFormerRunFail

export const CODEFORMER_FIDELITY_WEIGHTS = [0.9, 0.7, 0.5] as const
export type CodeFormerFidelityChoice = (typeof CODEFORMER_FIDELITY_WEIGHTS)[number]

export interface CodeFormerSaveResult {
  ok: boolean
  settings?: CodeFormerSettings
  error?: string
}
