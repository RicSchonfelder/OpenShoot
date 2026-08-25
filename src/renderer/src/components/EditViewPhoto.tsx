import { useEffect, useState } from 'react'
import { useT } from '../i18n/I18nContext'

/**
 * Foto em alta resolução exibida por inteiro (modo edição em tela grande).
 */
export default function EditViewPhoto({ photoId }: { photoId: number }) {
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

  return (
    <div className="editview-imgwrap">
      {src ? (
        <img src={src} alt="foto" className="editview-img" draggable={false} />
      ) : (
        <div className="editview-loading">{t('loupe.carregando')}</div>
      )}
    </div>
  )
}