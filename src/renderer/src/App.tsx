import { useCallback, useEffect, useState } from 'react'

type AppInfo = Awaited<ReturnType<typeof window.openshoot.appInfo>>

interface ChipProps {
  label: string
  value: string
}

function Chip({ label, value }: ChipProps) {
  return (
    <span className="chip">
      <span className="chip-label">{label}</span>
      <code>{value}</code>
    </span>
  )
}

export default function App() {
  const [coreMsg, setCoreMsg] = useState<string>('')
  const [name, setName] = useState<string>('fotógrafo')
  const [info, setInfo] = useState<AppInfo | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    window.openshoot.appInfo().then(setInfo).catch((e) => setError(String(e)))
  }, [])

  const runHello = useCallback(async () => {
    setBusy(true)
    setError(null)
    try {
      const msg = await window.openshoot.hello(name || 'fotógrafo')
      setCoreMsg(msg)
    } catch (e) {
      setError(String(e))
    } finally {
      setBusy(false)
    }
  }, [name])

  return (
    <div className="screen">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <span className="badge">Fase 0 · esqueleto</span>
      </header>

      <main className="content">
        <section className="panel">
          <h1>Ponte Electron ⇄ Rust</h1>
          <p className="muted">
            Prova de conceito: a UI (Electron/React) chama o core Rust (napi-rs)
            via IPC isolado — a mesma arquitetura do Aftershoot.
          </p>

          <div className="chips">
            {info && (
              <>
                <Chip label="Rust core" value={coreMsg ? 'carregado' : '…'} />
                <Chip label="SO" value={info.platform} />
                <Chip label="CPU" value={info.arch} />
                <Chip label="Electron" value={info.versions.electron ?? '?'} />
                <Chip label="Node" value={info.versions.node ?? '?'} />
                <Chip label="Chrome" value={info.versions.chrome ?? '?'} />
              </>
            )}
          </div>

          <div className="row">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Seu nome"
              aria-label="Seu nome"
            />
            <button onClick={runHello} disabled={busy}>
              {busy ? 'Chamando Rust…' : 'Chamar Rust'}
            </button>
          </div>

          {coreMsg && <pre className="output">{coreMsg}</pre>}
          {error && <pre className="output error">{error}</pre>}
        </section>
      </main>
    </div>
  )
}
