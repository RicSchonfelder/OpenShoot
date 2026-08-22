import { useEffect, useRef, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'
import { useT } from '../i18n/I18nContext'

interface LoupeViewProps {
  photos: PhotoMeta[]
  currentIndex: number
  onNavigate: (index: number) => void
  onApplyRating: (rating: number, advance: boolean) => void
  onClose: () => void
}

interface SelRect {
  x0: number
  y0: number
  x1: number
  y1: number
}

/**
 * Modo Loupe/Review: mostra a foto GRANDE, navega com setas, e P/X/U aplicam
 * rating e avançam — o fluxo de culling rápido estila referência externa/Lightroom.
 * Suporta seleção por arrasto para remover distrações (patch).
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
  const [preview, setPreview] = useState<string | null>(null)
  const [sel, setSel] = useState<SelRect | null>(null)
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null)
  const [showFaces, setShowFaces] = useState(false)
  const [faces, setFaces] = useState<number[][]>([])
  const stageRef = useRef<HTMLDivElement | null>(null)
  const imgRef = useRef<HTMLImageElement | null>(null)
  const photo = photos[currentIndex]
  const total = photos.length

  // Carrega faces (moldura) quando ativado.
  useEffect(() => {
    if (!showFaces || !photo) {
      setFaces([])
      return
    }
    let active = true
    window.openshoot
      .detectFacesInPhoto(photo.id)
      .then((res) => {
        if (active) setFaces(res.faces ?? [])
      })
      .catch(() => {})
    return () => {
      active = false
    }
  }, [showFaces, photo?.id])

  // Carrega a foto atual em alta resolução (preview grande).
  useEffect(() => {
    let active = true
    setSrc(null)
    setPreview(null)
    setSel(null)
    if (!photo) return
    setBusy(true)
    // Usa o thumbnail em alta resolução (máx ~2000px) para o loupe.
    window.openshoot
      .thumbForPhoto(photo.id, 2000)
      .then((t) => {
        if (active && t) setSrc(t)
      })
      .finally(() => active && setBusy(false))
    return () => {
      active = false
    }
  }, [photo?.id])

  // Coordenadas normalizadas (0..1) a partir de um evento de mouse no stage.
  const normCoords = (e: React.MouseEvent): { x: number; y: number } | null => {
    const img = imgRef.current
    const stage = stageRef.current
    if (!img || !stage) return null
    const rect = img.getBoundingClientRect()
    if (rect.width === 0 || rect.height === 0) return null
    return {
      x: (e.clientX - rect.left) / rect.width,
      y: (e.clientY - rect.top) / rect.height
    }
  }

  const onMouseDown = (e: React.MouseEvent) => {
    const p = normCoords(e)
    if (!p) return
    setDragStart(p)
    setSel({ x0: p.x, y0: p.y, x1: p.x, y1: p.y })
  }

  const onMouseMove = (e: React.MouseEvent) => {
    if (!dragStart) return
    const p = normCoords(e)
    if (!p) return
    setSel({ x0: dragStart.x, y0: dragStart.y, x1: p.x, y1: p.y })
  }

  const onMouseUp = () => {
    setDragStart(null)
  }

  const removeSelected = () => {
    if (!photo || !sel) return
    const x0 = Math.min(sel.x0, sel.x1)
    const x1 = Math.max(sel.x0, sel.x1)
    const y0 = Math.min(sel.y0, sel.y1)
    const y1 = Math.max(sel.y0, sel.y1)
    if (x1 - x0 < 0.01 || y1 - y0 < 0.01) return
    setBusy(true)
    window.openshoot
      .inpaintPhoto(photo.id, [x0, y0, x1, y1], 600)
      .then((t) => {
        if (t) {
          setPreview(t)
          setSel(null)
        }
      })
      .catch(() => {})
      .finally(() => setBusy(false))
  }

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

  const showImg = preview ?? src

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
        <button
          className={`loupe-faces ${showFaces ? 'active' : ''}`}
          onClick={() => setShowFaces((v) => !v)}
        >
          {t('loupe.mostrarRostos')}
        </button>
      </div>

      <div
        className="loupe-stage"
        ref={stageRef}
        onMouseDown={onMouseDown}
        onMouseMove={onMouseMove}
        onMouseUp={onMouseUp}
        onMouseLeave={onMouseUp}
      >
        {showImg ? (
          <img
            ref={imgRef}
            src={showImg}
            alt={photo.fileName}
            className="loupe-img"
            draggable={false}
          />
        ) : (
          <div className="loupe-loading">
            {busy ? t('loupe.carregando') : t('loupe.semPreview')}
          </div>
        )}
        {sel && (
          <div
            className="loupe-sel"
            style={{
              left: `${Math.min(sel.x0, sel.x1) * 100}%`,
              top: `${Math.min(sel.y0, sel.y1) * 100}%`,
              width: `${Math.abs(sel.x1 - sel.x0) * 100}%`,
              height: `${Math.abs(sel.y1 - sel.y0) * 100}%`
            }}
          />
        )}
        {showFaces &&
          faces.map((f, i) => (
            <div
              key={i}
              className="loupe-facebox"
              style={{
                left: `${f[0] * 100}%`,
                top: `${f[1] * 100}%`,
                width: `${(f[2] - f[0]) * 100}%`,
                height: `${(f[3] - f[1]) * 100}%`
              }}
            />
          ))}
        {photo.cullScore != null && (
          <span className="loupe-score">score {Math.round(photo.cullScore)}</span>
        )}
      </div>

      <div className="loupe-bottombar">
        <button
          onClick={removeSelected}
          disabled={!sel || busy}
          className="loupe-patch"
          title={t('loupe.patchHint')}
        >
          {t('loupe.patch')}
        </button>
        <span dangerouslySetInnerHTML={{ __html: t('loupe.atalhos') }} />
      </div>
    </div>
  )
}
