import { useCallback, useEffect, useRef, useState } from 'react'
import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent } from 'react'
import { useT } from '../i18n/I18nContext'
import type { PersistedFace } from '../../../types/photo'

type EditCompareMode = 'original' | 'modified' | 'side-by-side' | 'slider'

interface ContainRect {
  x: number
  y: number
  w: number
  h: number
}

function computeContainRect(
  containerW: number,
  containerH: number,
  naturalW: number,
  naturalH: number
): ContainRect {
  if (naturalW <= 0 || naturalH <= 0 || containerW <= 0 || containerH <= 0) {
    return { x: 0, y: 0, w: containerW, h: containerH }
  }
  const imgAspect = naturalW / naturalH
  const boxAspect = containerW / containerH
  let w: number, h: number
  if (imgAspect > boxAspect) {
    w = containerW
    h = containerW / imgAspect
  } else {
    h = containerH
    w = containerH * imgAspect
  }
  return { x: (containerW - w) / 2, y: (containerH - h) / 2, w, h }
}

interface EditViewPhotoProps {
  photoId: number
  modifiedSrc?: string | null
  compareMode: EditCompareMode
  onCompareModeChange: (mode: EditCompareMode) => void
  photoFaces?: PersistedFace[]
}

export default function EditViewPhoto({ photoId, modifiedSrc, compareMode, onCompareModeChange, photoFaces = [] }: EditViewPhotoProps) {
  const { t } = useT()
  const [src, setSrc] = useState<string | null>(null)
  const [naturalSize, setNaturalSize] = useState<{ w: number; h: number } | null>(null)
  const [zoom, setZoom] = useState(1)
  const [splitPosition, setSplitPosition] = useState(50)
  const [panningPane, setPanningPane] = useState<'original' | 'modified' | null>(null)
  const [pan, setPan] = useState({ x: 0, y: 0 })
  const [showPeople, setShowPeople] = useState(false)
  const panRef = useRef({ x: 0, y: 0 })
  const panDragRef = useRef<{ x: number; y: number; originX: number; originY: number } | null>(null)

  const containerRef = useRef<HTMLDivElement | null>(null)
  const modifiedContainerRef = useRef<HTMLDivElement | null>(null)
  const imgElRef = useRef<HTMLImageElement | null>(null)
  const modifiedImgElRef = useRef<HTMLImageElement | null>(null)

  const [originalContain, setOriginalContain] = useState<ContainRect>({ x: 0, y: 0, w: 0, h: 0 })
  const [modifiedContain, setModifiedContain] = useState<ContainRect>({ x: 0, y: 0, w: 0, h: 0 })

  useEffect(() => {
    let active = true
    setSrc(null)
    setNaturalSize(null)
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

  const measureOriginal = useCallback(() => {
    const containerEl = containerRef.current
    const imgEl = imgElRef.current
    if (!containerEl || !imgEl) return
    setOriginalContain(computeContainRect(containerEl.clientWidth, containerEl.clientHeight, imgEl.naturalWidth, imgEl.naturalHeight))
  }, [])

  const measureModified = useCallback(() => {
    const containerEl = modifiedContainerRef.current
    const imgEl = modifiedImgElRef.current
    if (!containerEl || !imgEl) return
    setModifiedContain(computeContainRect(containerEl.clientWidth, containerEl.clientHeight, imgEl.naturalWidth, imgEl.naturalHeight))
  }, [])

  useEffect(() => {
    const containers = [containerRef.current, modifiedContainerRef.current]
    if (!containers[0] && !containers[1]) return

    const ro = new ResizeObserver(() => {
      measureOriginal()
      measureModified()
    })
    for (const c of containers) {
      if (c) ro.observe(c)
    }
    return () => ro.disconnect()
  }, [measureOriginal, measureModified, compareMode, src, modifiedSrc])

  const syncPane = (pane: 'original' | 'modified') => {
    const from = pane === 'original' ? containerRef.current : modifiedContainerRef.current
    const to = pane === 'original' ? modifiedContainerRef.current : containerRef.current
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

  const handleZoom11 = useCallback(() => {
    const activeContain = compareMode === 'modified' ? modifiedContain : originalContain
    if (compareMode !== 'slider' && activeContain.w > 0 && naturalSize && naturalSize.w > 0) {
      const pixelZoom = naturalSize.w / activeContain.w
      setZoom(Math.min(8, Math.max(1, pixelZoom)))
    }
  }, [compareMode, originalContain, modifiedContain, naturalSize])

  const handleWheel = (event: ReactWheelEvent<HTMLDivElement>) => {
    event.preventDefault()
    changeZoom(zoom * Math.exp(-event.deltaY * 0.0015))
  }

  const facesTransform = `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)`

  const renderFacesOverlay = (faces: PersistedFace[]) => {
    if (!showPeople || faces.length === 0) return null
    return (
      <div
        className="editview-faces-overlay"
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          pointerEvents: 'none',
          zIndex: 1,
        }}
      >
        {faces.map((face) => {
          const [x1, y1, x2, y2] = face.bbox
          return (
            <div key={face.id} className="editview-face-bbox" style={{
              position: 'absolute',
              left: `${x1 * 100}%`,
              top: `${y1 * 100}%`,
              width: `${(x2 - x1) * 100}%`,
              height: `${(y2 - y1) * 100}%`,
            }}>
              <span className="editview-face-label">{face.group_name}</span>
            </div>
          )
        })}
      </div>
    )
  }

  const renderMediaLayer = (
    image: string | null | undefined,
    alt: string,
    setImgEl: (el: HTMLImageElement | null) => void,
    faces: PersistedFace[],
    contain: ContainRect,
    onLoadExtra?: () => void
  ) => (
    <div className="editview-media-layer" style={{
      position: 'absolute',
      left: contain.x,
      top: contain.y,
      width: contain.w,
      height: contain.h,
      overflow: 'hidden',
      transform: facesTransform,
      transformOrigin: 'top left',
    }}>
      {image ? (
        <img
          ref={setImgEl}
          src={image}
          alt={alt}
          className="editview-img"
          draggable={false}
          onLoad={(e) => {
            const img = e.currentTarget
            if (img.naturalWidth > 0 && img.naturalHeight > 0) {
              setNaturalSize({ w: img.naturalWidth, h: img.naturalHeight })
            }
            onLoadExtra?.()
          }}
          style={{
            width: '100%',
            height: '100%',
            objectFit: 'contain',
          }}
        />
      ) : (
        <div className="editview-compare-empty">{t('edit.compareEmpty')}</div>
      )}
      {renderFacesOverlay(faces)}
    </div>
  )

  const renderPane = (
    pane: 'original' | 'modified',
    image: string | null | undefined,
    alt: string,
    label: string,
    setImgEl: (el: HTMLImageElement | null) => void,
    faces: PersistedFace[],
    contain: ContainRect,
    onLoadExtra?: () => void
  ) => (
    <div
      ref={pane === 'original' ? containerRef : modifiedContainerRef}
      className={`editview-image-viewport ${panningPane === pane ? 'is-panning' : ''}`}
      onScroll={() => compareMode === 'side-by-side' && syncPane(pane)}
      onPointerDown={(event) => startPan(pane, event)}
      onPointerMove={movePan}
      onPointerUp={stopPan}
      onPointerCancel={stopPan}
      style={{ position: 'relative' }}
    >
      <span>{label}</span>
      {renderMediaLayer(image, alt, setImgEl, faces, contain, onLoadExtra)}
    </div>
  )

  const setSplitFromPointer = (event: ReactPointerEvent<HTMLDivElement>) => {
    const rect = event.currentTarget.getBoundingClientRect()
    setSplitPosition(Math.min(100, Math.max(0, ((event.clientX - rect.left) / rect.width) * 100)))
  }

  return (
    <div className={`editview-imgwrap ${compareMode === 'side-by-side' ? 'compare' : ''}`} onWheel={handleWheel}>
      <div className="editview-canvas">
        {src ? (compareMode === 'side-by-side'
          ? <>
              {renderPane('original', src, t('edit.originalPhoto'), t('edit.original'), (el) => { imgElRef.current = el }, photoFaces, originalContain, measureOriginal)}
              {renderPane('modified', modifiedSrc ?? src, t('edit.modifiedPhoto'), modifiedSrc ? t('edit.modified') : t('edit.originalNoEdit'), (el) => { modifiedImgElRef.current = el }, photoFaces, modifiedContain, measureModified)}
            </>
          : compareMode === 'slider'
            ? <div className="editview-slider-comparison">
                <span>{t('edit.sliderComparison')}</span>
                <div className="editview-slider-viewport" onPointerDown={(event) => { event.currentTarget.setPointerCapture(event.pointerId); setSplitFromPointer(event) }} onPointerMove={(event) => { if (event.buttons === 1) setSplitFromPointer(event) }}>
                  <img className="editview-slider-before" src={src} alt={t('edit.originalPhoto')} style={{ transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)` }} />
                  <div className="editview-slider-after" style={{ clipPath: `inset(0 ${100 - splitPosition}% 0 0)` }}>
                    <img src={modifiedSrc ?? src} alt={t('edit.modifiedPhoto')} style={{ transform: `scale(${zoom}) translate(${pan.x}px, ${pan.y}px)` }} />
                  </div>
                  <div className="editview-slider-handle" style={{ left: `${splitPosition}%` }}><span>↔</span></div>
                </div>
              </div>
          : compareMode === 'modified'
            ? renderPane('modified', modifiedSrc ?? src, modifiedSrc ? t('edit.modifiedPhoto') : t('edit.originalPhoto'), modifiedSrc ? t('edit.modified') : t('edit.originalNoEdit'), (el) => { modifiedImgElRef.current = el }, photoFaces, modifiedContain, measureModified)
            : renderPane('original', src, t('edit.originalPhoto'), t('edit.original'), (el) => { imgElRef.current = el }, photoFaces, originalContain, measureOriginal))
          : <div className="editview-loading">{t('loupe.carregando')}</div>}
      </div>
      <div className="editview-bottom-toolbar">
        <div className="editview-modebar" role="group" aria-label={t('edit.compareMode')}>
          <button type="button" onClick={() => onCompareModeChange('original')} className={compareMode === 'original' ? 'active' : ''}>{t('edit.original')}</button>
          <button type="button" onClick={() => onCompareModeChange('modified')} className={compareMode === 'modified' ? 'active' : ''}>{t('edit.modified')}</button>
          <button type="button" onClick={() => onCompareModeChange('side-by-side')} className={compareMode === 'side-by-side' ? 'active' : ''}>{t('edit.sideBySide')}</button>
          <button type="button" onClick={() => onCompareModeChange('slider')} className={compareMode === 'slider' ? 'active' : ''}>{t('edit.slider')}</button>
        </div>
        <div className="editview-people-toggle">
          <button
            type="button"
            onClick={() => setShowPeople((v) => !v)}
            className={showPeople ? 'active' : ''}
            disabled={compareMode === 'slider' || photoFaces.length === 0}
            title={compareMode === 'slider' ? t('people.overlayUnavailableSlider') : photoFaces.length === 0 ? t('people.semRostos') : showPeople ? t('people.ocultarPessoas') : t('people.mostrarPessoas')}
          >
            {showPeople ? t('people.ocultarPessoas') : t('people.mostrarPessoas')}
          </button>
        </div>
        <div className="editview-zoombar">
          <button type="button" onClick={() => changeZoom(1)} className={zoom === 1 ? 'active' : ''}>{t('edit.zoomFit')}</button>
          <button type="button" onClick={() => changeZoom(zoom - 0.25)} disabled={zoom <= 1}>−</button>
          <span>{Math.round(zoom * 100)}%</span>
          <button type="button" onClick={() => changeZoom(zoom + 0.25)} disabled={zoom >= 8}>+</button>
          <button type="button" onClick={handleZoom11} disabled={compareMode === 'slider'} title={compareMode === 'slider' ? t('people.overlayUnavailableSlider') : t('edit.zoom11')}>1:1</button>
        </div>
      </div>
    </div>
  )
}
