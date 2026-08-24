import { useState } from 'react'
import { useT } from '../i18n/I18nContext'

interface GalleryExportProps {
  ids: number[]
  onClose: () => void
}

export default function GalleryExport({ ids, onClose }: GalleryExportProps) {
  const { t } = useT()
  const [title, setTitle] = useState('')
  const [destDir, setDestDir] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const pickDest = () => {
    window.openshoot.pickExportFolder().then((d) => d && setDestDir(d))
  }

  const run = async () => {
    if (!destDir) return
    setBusy(true)
    setError(null)
    try {
      const res = await window.openshoot.createWebGallery(
        ids,
        destDir,
        title.trim() || t('galleryweb.tituloPadrao')
      )
      if (res.ok) {
        onClose()
      } else {
        setError(res.error ?? 'erro')
      }
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="dialog-overlay">
      <div className="dialog export-dialog">
        <h3>{t('galleryweb.titulo', { n: ids.length })}</h3>
        <p className="edit-hint">{t('galleryweb.hint')}</p>

        <div className="export-section">
          <h4>{t('galleryweb.nomeGaleria')}</h4>
          <input
            type="text"
            value={title}
            placeholder={t('galleryweb.nomePlaceholder')}
            onChange={(e) => setTitle(e.target.value)}
            style={{ width: '100%' }}
          />
        </div>

        <div className="export-section">
          <h4>{t('galleryweb.local')}</h4>
          <div className="export-dest">
            <span className="export-dest-path">{destDir || t('galleryweb.semDestino')}</span>
            <button onClick={pickDest} className="ghost">
              {t('galleryweb.escolher')}
            </button>
          </div>
        </div>

        {error && (
          <p className="edit-hint" role="alert">
            {error}
          </p>
        )}

        <div className="import-actions">
          <button onClick={onClose} className="ghost">
            {t('dialog.cancel')}
          </button>
          <button onClick={run} disabled={busy || !destDir} className="primary">
            {busy ? t('galleryweb.gerando') : t('galleryweb.gerar')}
          </button>
        </div>
      </div>
    </div>
  )
}
