import { useState } from 'react'
import { useT } from '../i18n/I18nContext'

interface ExportViewProps {
  photos: Array<{ id: number; path: string; filename?: string }>
  onClose: () => void
}

export default function ExportView({ photos, onClose }: ExportViewProps) {
  const { t } = useT()
  const [format, setFormat] = useState<'jpeg' | 'png' | 'tiff'>('jpeg')
  const [quality, setQuality] = useState(95)
  const [colorProfile, setColorProfile] = useState<'srgb' | 'display-p3' | 'adobe-rgb'>('srgb')
  const [resize, setResize] = useState<number | null>(null)
  const [naming, setNaming] = useState('{original}')
  const [destDir, setDestDir] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)

  const pickDest = () => {
    window.openshoot.pickExportFolder().then((d) => d && setDestDir(d))
  }

  const run = async () => {
    if (!destDir || photos.length === 0) return
    setBusy(true)
    setProgress({ done: 0, total: photos.length })
    try {
      const ids = photos.map((p) => p.id)
      const res = await window.openshoot.exportPhotos(ids, destDir, format === 'tiff' ? 'jpeg' : format, quality, colorProfile === 'adobe-rgb' ? 'srgb' : colorProfile, naming)
      if (res.ok) {
        alert(t('export.feito', { n: res.exported ?? 0 }))
      } else {
        alert(res.error ?? 'erro')
      }
      onClose()
    } finally {
      setBusy(false)
      setProgress(null)
    }
  }

  return (
    <div className="export-view">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <h2>{t('export.titulo', { n: photos.length })}</h2>
        <div className="topbar-right">
          <button onClick={onClose} className="ghost">
            ← {t('app.voltarGaleria')}
          </button>
        </div>
      </header>
      <main className="content export-content">
        <div className="export-section">
          <h3>{t('export.local')}</h3>
          <div className="export-dest">
            <span className="export-dest-path">{destDir || t('export.semDestino')}</span>
            <button onClick={pickDest} className="ghost">
              {t('export.escolher')}
            </button>
          </div>
        </div>

        <div className="export-section">
          <h3>{t('export.config')}</h3>
          <label className="edit-slider">
            <span>
              {t('export.tipoImagem')}
              <em>{format.toUpperCase()}</em>
            </span>
            <div className="export-formats">
              {(['jpeg', 'png', 'tiff'] as const).map((f) => (
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
              <em>{colorProfile === 'srgb' ? 'sRGB' : colorProfile === 'display-p3' ? 'Display P3' : 'Adobe RGB'}</em>
            </span>
            <select
              value={colorProfile}
              onChange={(e) => setColorProfile(e.target.value as 'srgb' | 'display-p3' | 'adobe-rgb')}
            >
              <option value="srgb">sRGB</option>
              <option value="display-p3">Display P3</option>
              <option value="adobe-rgb">Adobe RGB</option>
            </select>
          </label>
          <label className="edit-slider">
            <span>{t('export.redimensionar')}</span>
            <input
              type="number"
              value={resize ?? ''}
              onChange={(e) => setResize(e.target.value ? Number(e.target.value) : null)}
              placeholder={t('export.semRedimensionar')}
              min={100}
              max={10000}
            />
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
        </div>

        {progress && (
          <div className="export-progress">
            <span>{t('export.exportando')} {progress.done}/{progress.total}</span>
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{ width: `${Math.round((progress.done / progress.total) * 100)}%` }}
              />
            </div>
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
      </main>
    </div>
  )
}
