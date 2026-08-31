import { useEffect, useRef, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'
import { useT } from '../i18n/I18nContext'
import SettingsControl from './SettingsControl'
import WorkspaceNav, { type WorkspaceSection } from './WorkspaceNav'

interface LoupeViewProps {
  photos: PhotoMeta[]
  currentIndex: number
  onNavigate: (index: number) => void
  onApplyRating: (rating: number, advance: boolean) => void
  onClose: () => void
  onNavigateWorkspace: (section: WorkspaceSection) => void
}

interface SelRect {
  x0: number
  y0: number
  x1: number
  y1: number
}

const ZOOM_MAX = 8
const ZOOM_100 = 2.5
const ZOOM_STEP = 1.25

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
  onClose,
  onNavigateWorkspace
}: LoupeViewProps) {
  const { t } = useT()
  const [src, setSrc] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [preview, setPreview] = useState<string | null>(null)
  const [sel, setSel] = useState<SelRect | null>(null)
  const [dragStart, setDragStart] = useState<{ x: number; y: number } | null>(null)
  const [showFaces, setShowFaces] = useState(false)
  const [faces, setFaces] = useState<number[][]>([])
  const [zoom, setZoom] = useState(1)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [panning, setPanning] = useState(false)
  const zoomRef = useRef(1)
  const panRef = useRef({ x: 0, y: 0 })
  const panDragRef = useRef<{ sx: number; sy: number; ox: number; oy: number } | null>(null)
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
    zoomRef.current = 1
    panRef.current = { x: 0, y: 0 }
    setZoom(1)
    setPan({ x: 0, y: 0 })
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
    if (zoomRef.current > 1) {
      panDragRef.current = {
        sx: e.clientX,
        sy: e.clientY,
        ox: panRef.current.x,
        oy: panRef.current.y
      }
      setPanning(true)
      return
    }
    const p = normCoords(e)
    if (!p) return
    setDragStart(p)
    setSel({ x0: p.x, y0: p.y, x1: p.x, y1: p.y })
  }

  const onMouseMove = (e: React.MouseEvent) => {
    const pd = panDragRef.current
    if (pd) {
      const z = zoomRef.current
      const np = {
        x: pd.ox + (e.clientX - pd.sx) / z,
        y: pd.oy + (e.clientY - pd.sy) / z
      }
      panRef.current = np
      setPan(np)
      return
    }
    if (!dragStart) return
    const p = normCoords(e)
    if (!p) return
    setSel({ x0: dragStart.x, y0: dragStart.y, x1: p.x, y1: p.y })
  }

  const onMouseUp = () => {
    panDragRef.current = null
    setPanning(false)
    setDragStart(null)
  }

  // Zoom ancorado no cursor (wheel): mantém o ponto sob o mouse fixo.
  const zoomAt = (clientX: number, clientY: number, z2raw: number) => {
    const img = imgRef.current
    const stage = stageRef.current
    if (!img || !stage) return
    const z1 = zoomRef.current
    const z2 = Math.min(ZOOM_MAX, Math.max(1, z2raw))
    if (z2 === z1) return
    if (z2 <= 1) {
      zoomRef.current = 1
      panRef.current = { x: 0, y: 0 }
      setZoom(1)
      setPan({ x: 0, y: 0 })
      return
    }
    let sx = 0
    let sy = 0
    if (Number.isFinite(clientX) && Number.isFinite(clientY)) {
      const stageRect = stage.getBoundingClientRect()
      const mx = clientX - stageRect.left - img.offsetLeft
      const my = clientY - stageRect.top - img.offsetTop
      sx = mx - img.offsetWidth / 2
      sy = my - img.offsetHeight / 2
    }
    const np = {
      x: panRef.current.x + sx * (1 / z2 - 1 / z1),
      y: panRef.current.y + sy * (1 / z2 - 1 / z1)
    }
    zoomRef.current = z2
    panRef.current = np
    setZoom(z2)
    setPan(np)
  }

  const fitView = () => {
    zoomRef.current = 1
    panRef.current = { x: 0, y: 0 }
    setZoom(1)
    setPan({ x: 0, y: 0 })
  }

  // Zoom ancorado no centro da imagem (botões +/−/100%).
  const zoomCentered = (z2raw: number) => {
    zoomAt(NaN, NaN, z2raw)
  }

  // Wheel no stage: zoom centrado no cursor (listener nativo não-passivo).
  useEffect(() => {
    const stage = stageRef.current
    if (!stage) return
    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      zoomAt(e.clientX, e.clientY, zoomRef.current * Math.exp(-e.deltaY * 0.0015))
    }
    stage.addEventListener('wheel', onWheel, { passive: false })
    return () => stage.removeEventListener('wheel', onWheel)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

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
      <div className="loupe-topbar workspace-topbar">
        <div className="workspace-left loupe-summary">
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
        </div>
        <WorkspaceNav active="cull" onNavigate={onNavigateWorkspace} />
        <div className="topbar-right workspace-actions loupe-actions">
        <SettingsControl />
        <button className="loupe-close" onClick={onClose}>✕ {t('loupe.fechar')} (Esc)</button>
        <button
          className={`loupe-faces ${showFaces ? 'active' : ''}`}
          onClick={() => setShowFaces((v) => !v)}
        >
          {t('loupe.mostrarRostos')}
        </button>
        </div>
      </div>

      <div
        className="loupe-stage"
        ref={stageRef}
        style={{ cursor: zoom > 1 ? (panning ? 'grabbing' : 'grab') : undefined }}
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
            style={
              zoom > 1
                ? {
                    transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)`
                  }
                : undefined
            }
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
        <div className="loupe-zoombar">
          <button
            type="button"
            className={`loupe-zoom-btn${zoom === 1 ? ' active' : ''}`}
            onClick={fitView}
            title={t('loupe.ajustar')}
          >
            {t('loupe.ajustar')}
          </button>
          <button
            type="button"
            className="loupe-zoom-btn"
            onClick={() => zoomCentered(ZOOM_100)}
            title={t('loupe.zoom100')}
          >
            {t('loupe.zoom100')}
          </button>
          <button
            type="button"
            className="loupe-zoom-btn"
            onClick={() => zoomCentered(zoomRef.current / ZOOM_STEP)}
            disabled={zoom <= 1}
            aria-label={t('loupe.menos')}
          >
            {t('loupe.menos')}
          </button>
          <button
            type="button"
            className="loupe-zoom-btn"
            onClick={() => zoomCentered(zoomRef.current * ZOOM_STEP)}
            disabled={zoom >= ZOOM_MAX}
            aria-label={t('loupe.mais')}
          >
            {t('loupe.mais')}
          </button>
          <span className="loupe-zoom-label">{Math.round(zoom * 100)}%</span>
        </div>
        <span dangerouslySetInnerHTML={{ __html: t('loupe.atalhos') }} />
      </div>
    </div>
  )
}
