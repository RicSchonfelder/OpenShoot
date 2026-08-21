import { useState } from 'react'
import { useT } from '../i18n/I18nContext'

interface ExportDialogProps {
  ids: number[]
  onClose: () => void
  onDone: (msg: string) => void
}

export default function ExportDialog({ ids, onClose, onDone }: ExportDialogProps) {
  const { t } = useT()
  const [format, setFormat] = useState<'jpeg' | 'png'>('jpeg')
  const [quality, setQuality] = useState(100)
  const [destDir, setDestDir] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const photoCount = ids.length

  const pickDest = () => {
    window.openshoot.pickExportFolder().then((d) => d && setDestDir(d))
  }

  const run = async () => {
    if (!destDir) return
    setBusy(true)
    try {
      const res = await window.openshoot.exportPhotos(ids, destDir, format, quality)
      if (res.ok) {
        onDone(t('export.feito', { n: res.exported ?? 0 }))
      } else {
        onDone(res.error ?? 'erro')
      }
      onClose()
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="dialog-overlay">
      <div className="dialog export-dialog">
        <h3>{t('export.titulo', { n: photoCount })}</h3>
        <p className="edit-hint">
          {photoCount === 1 ? t('export.dica1') : t('export.dicaN', { n: photoCount })}
        </p>

        <div className="export-section">
          <h4>{t('export.local')}</h4>
          <div className="export-dest">
            <span className="export-dest-path">{destDir || t('export.semDestino')}</span>
            <button onClick={pickDest} className="ghost">
              {t('export.escolher')}
            </button>
          </div>
        </div>

        <div className="export-section">
          <h4>{t('export.config')}</h4>
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
        </div>

        <div className="import-actions">
          <button onClick={onClose} className="ghost">
            {t('dialog.cancel')}
          </button>
          <button onClick={run} disabled={busy || !destDir} className="primary">
            {busy ? t('export.exportando') : t('export.exportar', { n: photoCount })}
          </button>
        </div>
      </div>
    </div>
  )
}