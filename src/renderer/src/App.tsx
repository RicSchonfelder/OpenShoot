import { useCallback, useEffect, useState } from 'react'
import Gallery from './components/Gallery'
import type { PhotoMeta } from '../../types/photo'

const PAGE_SIZE = 200

type Filter = 'all' | 'picks' | 'rejects' | 'unrated'

const FILTER_LABELS: Record<Filter, string> = {
  all: 'Todos',
  picks: 'Picks (★≥4)',
  rejects: 'Rejects (★≤1)',
  unrated: 'Sem avaliação'
}

export default function App() {
  const [photos, setPhotos] = useState<PhotoMeta[]>([])
  const [filter, setFilter] = useState<Filter>('all')
  const [scanning, setScanning] = useState(false)
  const [culling, setCulling] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [scanMsg, setScanMsg] = useState<string | null>(null)
  const [scanErrors, setScanErrors] = useState<string[]>([])
  const [error, setError] = useState<string | null>(null)

  const loadPhotos = useCallback(
    async (f: Filter = filter) => {
      try {
        const list = await window.openshoot.listPhotos('', f, 0, PAGE_SIZE)
        setPhotos(list.photos)
      } catch (e) {
        setError(String(e))
      }
    },
    [filter]
  )

  useEffect(() => {
    loadPhotos()
  }, [loadPhotos, filter])

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

  const exportXmp = useCallback(async () => {
    setError(null)
    setExporting(true)
    try {
      const res = await window.openshoot.exportAllXmp()
      if ('error' in res) {
        setError(String(res.error))
      } else {
        setScanMsg(
          `XMP exportado: ${res.exported} sidecars criados` +
            (res.errors > 0 ? `, ${res.errors} erros` : '')
        )
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setExporting(false)
    }
  }, [])

  return (
    <div className="app">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <div className="topbar-filters">
          {(Object.keys(FILTER_LABELS) as Filter[]).map((f) => (
            <button
              key={f}
              className={`filter-btn ${filter === f ? 'active' : ''}`}
              onClick={() => setFilter(f)}
            >
              {FILTER_LABELS[f]}
            </button>
          ))}
        </div>
        <div className="topbar-right">
          <button onClick={exportXmp} disabled={exporting || photos.length === 0} className="ghost">
            {exporting ? 'Exportando…' : 'Exportar XMP'}
          </button>
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
