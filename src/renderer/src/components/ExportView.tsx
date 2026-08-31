import { useState } from 'react'
import { useT } from '../i18n/I18nContext'
import SettingsControl from './SettingsControl'
import WorkspaceNav, { type WorkspaceSection } from './WorkspaceNav'

interface ExportViewProps {
  photos: Array<{ id: number; path: string; filename?: string }>
  scope: 'selection' | 'visible'
  onClose: () => void
  onNavigate: (section: WorkspaceSection) => void
}

export default function ExportView({ photos, scope, onClose, onNavigate }: ExportViewProps) {
  const { t } = useT()
  const [format, setFormat] = useState<'jpeg' | 'png'>('jpeg')
  const [quality, setQuality] = useState(95)
  const [colorProfile, setColorProfile] = useState<'srgb' | 'display-p3'>('srgb')
  const [naming, setNaming] = useState('{original}')
  const [destDir, setDestDir] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<{ ok: boolean; n: number; error?: string } | null>(null)

  const pickDest = () => {
    window.openshoot.pickExportFolder().then((d) => d && setDestDir(d))
  }

  const run = async () => {
    if (!destDir || photos.length === 0) return
    setBusy(true)
    setResult(null)
    try {
      const ids = photos.map((p) => p.id)
      const res = await window.openshoot.exportPhotos(ids, destDir, format, quality, colorProfile, naming)
      if (res.ok) {
        setResult({ ok: true, n: res.exported ?? 0 })
      } else {
        setResult({ ok: false, n: 0, error: res.error ?? 'erro' })
      }
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="export-view">
      <header className="topbar workspace-topbar export-topbar">
        <div className="workspace-left"><span className="logo">OpenShoot</span></div>
        <WorkspaceNav active="export" onNavigate={onNavigate} />
        <div className="topbar-right workspace-actions">
          <SettingsControl />
          <button onClick={onClose} className="ghost">
            {t('app.voltarGaleria')}
          </button>
        </div>
      </header>
      <main className="content export-content">
        <div className="export-page-heading">
          <h1>{t('export.titulo', { n: photos.length })}</h1>
          <p>
            {scope === 'selection' ? t('export.exportScopeSelection') : t('export.exportScopeVisible')}
            {' '}{t('export.exportScopeHint')}
          </p>
        </div>
        <section className="export-section export-destination-section">
          <h3>{t('export.local')}</h3>
          <div className="export-dest">
            <span className="export-dest-path">{destDir || t('export.semDestino')}</span>
            <button onClick={pickDest} className="ghost">
              {t('export.escolher')}
            </button>
          </div>
        </section>

        <section className="export-section export-config-section">
          <h3>{t('export.config')}</h3>
          <label className="edit-slider">
            <span>
              {t('export.tipoImagem')}
              <em>{format.toUpperCase()}</em>
            </span>
            <div className="export-formats">
              {(['jpeg', 'png'] as const).map((f) => (
                <button
                  key={f}
                  className={`${format === f ? 'active' : ''}`}
                  onClick={() => setFormat(f)}
                >
                  {f.toUpperCase()}
                </button>
              ))}
            </div>
          </label>
          {format === 'jpeg' && (
            <label className="edit-slider">
              <span>
                {t('export.qualidade')}
                <em>{quality}%</em>
              </span>
              <input
                type="range"
                min={1}
                max={100}
                step={1}
                value={quality}
                onChange={(e) => setQuality(Number(e.target.value))}
              />
            </label>
          )}
          <label className="edit-slider">
            <span>
              {t('export.espacoCor')}
                <em>{colorProfile === 'srgb' ? 'sRGB' : 'Display P3'}</em>
            </span>
            <select
              value={colorProfile}
              onChange={(e) => setColorProfile(e.target.value as 'srgb' | 'display-p3')}
            >
              <option value="srgb">sRGB</option>
              <option value="display-p3">Display P3 (aproximação)</option>
            </select>
          </label>
          <label className="edit-slider">
            <span>{t('export.nomeacao')}</span>
            <input
              type="text"
              value={naming}
              onChange={(e) => setNaming(e.target.value)}
              placeholder="{original}"
            />
          </label>
          <div className="export-formats">
            {([
              ['{original}', t('export.namingOriginal')],
              ['{n}_{original}', t('export.namingContador')],
              ['{date}_{original}', t('export.namingData')]
            ] as const).map(([pattern, label]) => (
              <button
                key={pattern}
                className={`${naming === pattern ? 'active' : ''}`}
                onClick={() => setNaming(pattern)}
              >
                {label}
              </button>
            ))}
          </div>
        </section>

        <footer className="export-footer">
          {busy && <div className="export-progress">{t('export.exportando')}</div>}
          {result && (
            <div className={`toast ${result.ok ? 'success' : 'error'}`}>
              {result.ok
                ? t('export.feito', { n: result.n })
                : result.error ?? 'erro'}
            </div>
          )}
          <div className="import-actions">
            <button onClick={onClose} className="ghost">
              {t('dialog.cancel')}
            </button>
            <button onClick={run} disabled={busy || !destDir || photos.length === 0} className="primary">
              {busy ? t('export.exportando') : t('export.exportar', { n: photos.length })}
            </button>
          </div>
        </footer>
      </main>
    </div>
  )
}
