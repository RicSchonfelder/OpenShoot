import { useEffect, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'
import { useT } from '../i18n/I18nContext'

interface LoupeViewProps {
  photos: PhotoMeta[]
  currentIndex: number
  onNavigate: (index: number) => void
  onApplyRating: (rating: number, advance: boolean) => void
  onClose: () => void
}

/**
 * Modo Loupe/Review: mostra a foto GRANDE, navega com setas, e P/X/U aplicam
 * rating e avançam — o fluxo de culling rápido estilo AfterShoot/Lightroom.
 */
export default function LoupeView({
  photos,
  currentIndex,
  onNavigate,
  onApplyRating,
  onClose
}: LoupeViewProps) {
  const { t } = useT()
  const [src, setSrc] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const photo = photos[currentIndex]
  const total = photos.length

  // Carrega a foto atual em alta resolução (preview grande).
  useEffect(() => {
    let active = true
    setSrc(null)
    if (!photo) return
    setBusy(true)
    // Usa o thumbnail em alta resolução (máx ~1200px) para o loupe.
    window.openshoot
      .thumbForPhoto(photo.id, 1200)
      .then((t) => {
        if (active && t) setSrc(t)
      })
      .finally(() => active && setBusy(false))
    return () => {
      active = false
    }
  }, [photo?.id])

  // Atalhos do loupe.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return
      switch (e.key) {
        case 'ArrowRight':
        case 'ArrowDown':
          e.preventDefault()
          onNavigate(Math.min(total - 1, currentIndex + 1))
          break
        case 'ArrowLeft':
        case 'ArrowUp':
          e.preventDefault()
          onNavigate(Math.max(0, currentIndex - 1))
          break
        case 'p':
        case 'P':
          onApplyRating(5, true)
          break
        case 'x':
        case 'X':
          onApplyRating(1, true)
          break
        case 'u':
        case 'U':
          onApplyRating(0, true)
          break
        case '1':
        case '2':
        case '3':
        case '4':
        case '5':
          onApplyRating(Number(e.key), true)
          break
        case 'Escape':
          onClose()
          break
        case ' ':
          e.preventDefault()
          // espaço = próximo (recomendação de navegação em culling)
          onNavigate(Math.min(total - 1, currentIndex + 1))
          break
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [currentIndex, total, onNavigate, onApplyRating, onClose])

  if (!photo) {
    return (
      <div className="loupe">
        <button className="loupe-close" onClick={onClose}>✕</button>
        <div className="loupe-empty">{t('loupe.semFotos')}</div>
      </div>
    )
  }

  const flags = {
    pick: photo.rating >= 4,
    reject: photo.rating >= 1 && photo.rating <= 2
  }

  return (
    <div className="loupe">
      <div className="loupe-topbar">
        <span className="loupe-counter">
          {t('loupe.contador', { current: currentIndex + 1, total, name: photo.fileName })}
        </span>
        <div className="loupe-flags">
          <span className={`flag flag-pick ${flags.pick ? 'on' : ''}`}>P</span>
          <span className={`flag flag-reject ${flags.reject ? 'on' : ''}`}>X</span>
          <div className="loupe-stars" role="radiogroup" aria-label={t('gallery.starRating')}>
            {[1, 2, 3, 4, 5].map((star) => (
              <button
                key={star}
                type="button"
                role="radio"
                aria-checked={photo.rating === star}
                aria-label={t('gallery.starN', { n: star })}
                className={`loupe-star ${star <= photo.rating ? 'on' : ''}`}
                onClick={() => onApplyRating(star === photo.rating ? 0 : star, false)}
              >
                ★
              </button>
            ))}
          </div>
        </div>
        <button className="loupe-close" onClick={onClose}>✕ {t('loupe.fechar')} (Esc)</button>
      </div>

      <div className="loupe-stage">
        {src ? (
          <img src={src} alt={photo.fileName} className="loupe-img" />
        ) : (
          <div className="loupe-loading">
            {busy ? t('loupe.carregando') : t('loupe.semPreview')}
          </div>
        )}
        {photo.cullScore != null && (
          <span className="loupe-score">score {Math.round(photo.cullScore)}</span>
        )}
      </div>

      <div className="loupe-bottombar">
        <span dangerouslySetInnerHTML={{ __html: t('loupe.atalhos') }} />
      </div>
    </div>
  )
}
