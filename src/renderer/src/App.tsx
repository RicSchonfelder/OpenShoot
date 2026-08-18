import { useCallback, useEffect, useState } from 'react'
import Gallery from './components/Gallery'
import type { PhotoMeta } from '../../types/photo'

const PAGE_SIZE = 200

export default function App() {
  const [photos, setPhotos] = useState<PhotoMeta[]>([])
  const [scanning, setScanning] = useState(false)
  const [culling, setCulling] = useState(false)
  const [scanMsg, setScanMsg] = useState<string | null>(null)
  const [scanErrors, setScanErrors] = useState<string[]>([])
  const [error, setError] = useState<string | null>(null)

  const loadPhotos = useCallback(async () => {
    try {
      const list = await window.openshoot.listPhotos('', 0, PAGE_SIZE)
      setPhotos(list.photos)
    } catch (e) {
      setError(String(e))
    }
  }, [])

  useEffect(() => {
    loadPhotos()
  }, [loadPhotos])

  const importFolder = useCallback(async () => {
    setError(null)
    setScanMsg(null)
    setScanErrors([])
    const dir = await window.openshoot.pickFolder()
    if (!dir) return
    setScanning(true)
    try {
      const res = await window.openshoot.scanFolder(dir)
      if ('error' in res) {
        setError(res.error as string)
      } else {
        setScanMsg(
          `Importado ${dir}: ${res.scanned} arquivos, +${res.added} novos, ` +
            `${res.updated} atualizados, ${res.errors.length} erros`
        )
        if (res.errors.length) setScanErrors(res.errors.slice(0, 20))
        await loadPhotos()
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setScanning(false)
    }
  }, [loadPhotos])

  const runCull = useCallback(async () => {
    setError(null)
    setCulling(true)
    try {
      const res = await window.openshoot.cullPhotos()
      if ('error' in res) {
        setError(String(res.error))
      } else {
        setScanMsg(
          `Culling: ${res.processed} fotos avaliadas, ${res.picks} picks, ` +
            `score médio ${res.avgScore.toFixed(1)}`
        )
        await loadPhotos()
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setCulling(false)
    }
  }, [loadPhotos])

  return (
    <div className="app">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <div className="topbar-right">
          <span className="badge">Fase 2 · culling</span>
          <button onClick={runCull} disabled={culling || photos.length === 0} className="primary">
            {culling ? 'Culling…' : 'Cull'}
          </button>
          <button onClick={importFolder} disabled={scanning}>
            {scanning ? 'Importando…' : 'Importar pasta'}
          </button>
        </div>
      </header>

      {scanMsg && <div className="toast">{scanMsg}</div>}
      {scanErrors.length > 0 && (
        <details className="scan-errors">
          <summary>{scanErrors.length} erro(s) ao importar</summary>
          <ul>
            {scanErrors.map((er, i) => (
              <li key={i}>{er}</li>
            ))}
          </ul>
        </details>
      )}
      {error && <div className="toast error">{error}</div>}

      <main className="content">
        <Gallery photos={photos} onRefresh={loadPhotos} />
      </main>
    </div>
  )
}
