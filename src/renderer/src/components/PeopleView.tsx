import { useCallback, useEffect, useState } from 'react'
import { useT } from '../i18n/I18nContext'

interface PeopleViewProps {
  onBack: () => void
}

interface PersonGroup {
  person_id: number
  count: number
  sample_path: string
  photo_ids: number[]
  photo_paths: string[]
}

function extractGroups(res: unknown): PersonGroup[] {
  if (Array.isArray(res)) return res as PersonGroup[]
  if (res && typeof res === 'object') {
    const obj = res as Record<string, unknown>
    if (typeof obj.error === 'string') throw new Error(obj.error)
    if (Array.isArray(obj.groups)) return obj.groups as PersonGroup[]
  }
  return []
}

export default function PeopleView({ onBack }: PeopleViewProps) {
  const { t } = useT()
  const [threshold, setThreshold] = useState(0.5)
  const [grouping, setGrouping] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [people, setPeople] = useState<PersonGroup[]>([])
  const [covers, setCovers] = useState<Record<number, string>>({})
  const [error, setError] = useState<string | null>(null)

  // Carrega a capa de cada pessoa (thumbnail da foto representativa).
  useEffect(() => {
    const missing = people.filter((p) => p.sample_path && !covers[p.person_id])
    missing.forEach((p) => {
      window.openshoot
        .thumbForPath(p.sample_path, 300)
        .then((t) => t && setCovers((c) => ({ ...c, [p.person_id]: t })))
        .catch(() => {})
    })
  }, [people, covers])

  const runGrouping = useCallback(async () => {
    setGrouping(true)
    setError(null)
    try {
      const res = await window.openshoot.groupBySimilarity(threshold)
      setPeople(extractGroups(res))
      setCovers({})
    } catch (e) {
      setError(String(e))
    } finally {
      setGrouping(false)
    }
  }, [threshold])

  const runExport = useCallback(async () => {
    const outDir = await window.openshoot.pickExportFolder()
    if (!outDir) return
    setExporting(true)
    setError(null)
    try {
      const res = await window.openshoot.exportPeopleToFolders(outDir, threshold)
      if (!res.ok) {
        setError(res.error ?? 'erro')
      } else {
        window.alert(t('people.exportado', { n: res.exported ?? 0, dir: res.out_dir ?? outDir }))
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setExporting(false)
    }
  }, [threshold, t])

  return (
    <div className="people">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <div className="topbar-right">
          <label className="people-threshold">
            <input
              type="range"
              min={0.3}
              max={0.8}
              step={0.01}
              value={threshold}
              onChange={(e) => setThreshold(Number(e.target.value))}
            />
            <em>{threshold.toFixed(2)}</em>
          </label>
          <button onClick={runGrouping} disabled={grouping || exporting}>
            {grouping ? t('people.agrupando') : t('people.agrupar')}
          </button>
          <button onClick={runExport} disabled={grouping || exporting || people.length === 0}>
            {exporting ? '…' : t('people.exportar')}
          </button>
          <button onClick={onBack} className="ghost back-albums">
            ← {t('people.voltar')}
          </button>
        </div>
      </header>

      {error && <div className="toast error">{error}</div>}

      <main className="people-body">
        {grouping ? (
          <div className="people-loading">
            <span className="people-spinner" />
            {t('people.agrupando')}
          </div>
        ) : people.length === 0 ? (
          <div className="people-empty">
            <div className="home-empty-title">{t('people.titulo')}</div>
            <p>{t('people.nenhuma')}</p>
          </div>
        ) : (
          <div className="people-grid">
            {people.map((p) => (
              <div key={p.person_id} className="person-card">
                <div className="person-cover">
                  {covers[p.person_id] ? (
                    <img src={covers[p.person_id]} alt={t('people.pessoa', { n: p.person_id + 1 })} />
                  ) : (
                    <div className="person-cover-empty">👤</div>
                  )}
                  <span className="person-count">{t('gallery.fotoCount', { n: p.count })}</span>
                </div>
                <div className="person-meta">
                  <span className="person-name">{t('people.pessoa', { n: p.person_id + 1 })}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  )
}
