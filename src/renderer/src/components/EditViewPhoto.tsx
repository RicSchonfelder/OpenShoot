import { useEffect, useState } from 'react'
import { useT } from '../i18n/I18nContext'

/**
 * Foto em alta resolução exibida por inteiro (modo edição em tela grande).
 */
export default function EditViewPhoto({ photoId, modifiedSrc, compareMode }: { photoId: number; modifiedSrc?: string | null; compareMode: 'single' | 'side-by-side' }) {
  const { t } = useT()
  const [src, setSrc] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    setSrc(null)
    window.openshoot
      .thumbForPhoto(photoId, 2400)
      .then((t) => {
        if (active && t) setSrc(t)
      })
      .catch(() => {})
    return () => {
      active = false
    }
  }, [photoId])

  const renderImage = (image: string | null | undefined, alt: string) => image
    ? <img src={image} alt={alt} className="editview-img" draggable={false} />
    : <div className="editview-compare-empty">Aplique uma modificação para comparar</div>

  return (
    <div className={`editview-imgwrap ${compareMode === 'side-by-side' ? 'compare' : ''}`}>
      {src ? (compareMode === 'side-by-side'
        ? <>
            <div className="editview-compare-pane"><span>Original</span>{renderImage(src, 'Foto original')}</div>
            <div className="editview-compare-pane"><span>Modificada</span>{renderImage(modifiedSrc, 'Foto modificada')}</div>
          </>
        : renderImage(modifiedSrc ?? src, modifiedSrc ? 'Foto modificada' : 'Foto original'))
        : <div className="editview-loading">{t('loupe.carregando')}</div>}
    </div>
  )
}
