import { useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent } from 'react'
import { useT } from '../i18n/I18nContext'

type EditCompareMode = 'original' | 'modified' | 'side-by-side' | 'slider'

/**
 * Foto em alta resolução exibida por inteiro (modo edição em tela grande).
 */
export default function EditViewPhoto({ photoId, modifiedSrc, compareMode, onCompareModeChange }: { photoId: number; modifiedSrc?: string | null; compareMode: EditCompareMode; onCompareModeChange: (mode: EditCompareMode) => void }) {
  const { t } = useT()
  const [src, setSrc] = useState<string | null>(null)
  const [zoom, setZoom] = useState(1)
  const [splitPosition, setSplitPosition] = useState(50)
  const [panningPane, setPanningPane] = useState<'original' | 'modified' | null>(null)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const originalPaneRef = useRef<HTMLDivElement | null>(null)
  const modifiedPaneRef = useRef<HTMLDivElement | null>(null)
  const panRef = useRef({ x: 0, y: 0 })
  const panDragRef = useRef<{ x: number; y: number; originX: number; originY: number } | null>(null)

  useEffect(() => {
    let active = true
    setSrc(null)
    setZoom(1)
    panRef.current = { x: 0, y: 0 }
    setPan({ x: 0, y: 0 })
    window.openshoot
      .thumbForPhoto(photoId, 4096)
      .then((t) => {
        if (active && t) setSrc(t)
      })
      .catch(() => {})
    return () => {
      active = false
    }
  }, [photoId])

  const syncPane = (pane: 'original' | 'modified') => {
    const from = pane === 'original' ? originalPaneRef.current : modifiedPaneRef.current
    const to = pane === 'original' ? modifiedPaneRef.current : originalPaneRef.current
    if (from && to) {
      to.scrollLeft = from.scrollLeft
      to.scrollTop = from.scrollTop
    }
  }

  const startPan = (pane: 'original' | 'modified', event: ReactPointerEvent<HTMLDivElement>) => {
    if (zoom <= 1) return
    event.currentTarget.setPointerCapture(event.pointerId)
    panDragRef.current = { x: event.clientX, y: event.clientY, originX: panRef.current.x, originY: panRef.current.y }
    setPanningPane(pane)
  }

  const movePan = (event: ReactPointerEvent<HTMLDivElement>) => {
    const drag = panDragRef.current
    if (!drag || !event.currentTarget.hasPointerCapture(event.pointerId)) return
    const next = { x: drag.originX + (event.clientX - drag.x) / zoom, y: drag.originY + (event.clientY - drag.y) / zoom }
    panRef.current = next
    setPan(next)
  }

  const stopPan = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
    panDragRef.current = null
    setPanningPane(null)
  }

  const changeZoom = (next: number) => {
    const value = Math.min(8, Math.max(1, next))
    setZoom(value)
    if (value === 1) {
      panRef.current = { x: 0, y: 0 }
      setPan({ x: 0, y: 0 })
    }
  }
  const handleWheel = (event: ReactWheelEvent<HTMLDivElement>) => {
    event.preventDefault()
    changeZoom(zoom * Math.exp(-event.deltaY * 0.0015))
  }

  const renderImage = (image: string | null | undefined, alt: string) => image
    ? <img src={image} alt={alt} className="editview-img" draggable={false} style={{ transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)` }} />
    : <div className="editview-compare-empty">Aplique uma modificação para comparar</div>

  const renderPane = (pane: 'original' | 'modified', image: string | null | undefined, alt: string, label: string) => (
    <div
      ref={pane === 'original' ? originalPaneRef : modifiedPaneRef}
      className={`editview-image-viewport ${panningPane === pane ? 'is-panning' : ''}`}
      onScroll={() => compareMode === 'side-by-side' && syncPane(pane)}
      onPointerDown={(event) => startPan(pane, event)}
      onPointerMove={movePan}
      onPointerUp={stopPan}
      onPointerCancel={stopPan}
    >
      <span>{label}</span>
      {renderImage(image, alt)}
    </div>
  )

  const setSplitFromPointer = (event: ReactPointerEvent<HTMLDivElement>) => {
    const rect = event.currentTarget.getBoundingClientRect()
    setSplitPosition(Math.min(100, Math.max(0, ((event.clientX - rect.left) / rect.width) * 100)))
  }

  return (
    <div className={`editview-imgwrap ${compareMode === 'side-by-side' ? 'compare' : ''}`} onWheel={handleWheel}>
      {src ? (compareMode === 'side-by-side'
        ? <>
            {renderPane('original', src, 'Foto original', 'Original')}
            {renderPane('modified', modifiedSrc ?? src, 'Foto modificada', modifiedSrc ? 'Modificada' : 'Original · sem edição')}
          </>
        : compareMode === 'slider'
          ? <div className="editview-slider-comparison">
              <span>Comparação</span>
              <div className="editview-slider-viewport" onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); setSplitFromPointer(event) }} onPointerMove={(event) => { if (event.buttons === 1) setSplitFromPointer(event) }}>
                <img className="editview-slider-before" src={src} alt="Original" style={{ transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)` }} />
                <div className="editview-slider-after" style={{ clipPath: `inset(0 ${100 - splitPosition}% 0 0)` }}>
                  <img src={modifiedSrc ?? src} alt="Modificada" style={{ transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)` }} />
                </div>
                <div className="editview-slider-handle" style={{ left: `${splitPosition}%` }}><span>↔</span></div>
              </div>
            </div>
        : compareMode === 'modified'
          ? renderPane('modified', modifiedSrc ?? src, modifiedSrc ? 'Foto modificada' : 'Foto original', modifiedSrc ? 'Modificada' : 'Original · sem edição')
          : renderPane('original', src, 'Foto original', 'Original'))
        : <div className="editview-loading">{t('loupe.carregando')}</div>}
      <div className="editview-bottom-toolbar">
        <div className="editview-modebar" role="group" aria-label="Modo de comparação">
          <button type="button" onClick={() => onCompareModeChange('original')} className={compareMode === 'original' ? 'active' : ''}>Original</button>
          <button type="button" onClick={() => onCompareModeChange('modified')} className={compareMode === 'modified' ? 'active' : ''}>Modificada</button>
          <button type="button" onClick={() => onCompareModeChange('side-by-side')} className={compareMode === 'side-by-side' ? 'active' : ''}>Lado a lado</button>
          <button type="button" onClick={() => onCompareModeChange('slider')} className={compareMode === 'slider' ? 'active' : ''}>Comparar</button>
        </div>
        <div className="editview-zoombar">
          <button type="button" onClick={() => changeZoom(1)} className={zoom === 1 ? 'active' : ''}>Ajustar</button>
          <button type="button" onClick={() => changeZoom(zoom - 0.25)} disabled={zoom <= 1}>−</button>
          <span>{Math.round(zoom * 100)}%</span>
          <button type="button" onClick={() => changeZoom(zoom + 0.25)} disabled={zoom >= 8}>+</button>
          <button type="button" onClick={() => changeZoom(1)}>1:1</button>
        </div>
      </div>
    </div>
  )
}
