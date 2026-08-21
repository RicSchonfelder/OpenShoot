import { useEffect, useRef, useState } from 'react'
import { Grid } from 'react-window'
import type { PhotoMeta } from '../../../types/photo'
import { getThumb, setThumb } from '../thumbCache'
import { useT } from '../i18n/I18nContext'

interface GalleryProps {
  photos: PhotoMeta[]
  onRefresh: () => void
  selectedIds: Set<number>
  anchorId: number | null
  onSelect: (id: number, opts: { extend: boolean; toggle: boolean }) => void
  onActivate: (id: number) => void
  onRate: (id: number, rating: number) => void
}

const CELL_SIZE = 200
const ROW_HEIGHT = CELL_SIZE + 40
const DEFAULT_WIDTH = 1200

function computeCols(width: number): number {
  return Math.max(2, Math.floor(width / CELL_SIZE))
}

interface CellData {
  photos: PhotoMeta[]
  cols: number
  maxDim: number
  selectedIds: Set<number>
  onSelect: (id: number, opts: { extend: boolean; toggle: boolean }) => void
  onActivate: (id: number) => void
  onRate: (id: number, rating: number) => void
}

function Thumb({
  photo,
  maxDim,
  selected,
  onSelect,
  onActivate,
  onRate
}: {
  photo: PhotoMeta
  maxDim: number
  selected: boolean
  onSelect: (id: number, opts: { extend: boolean; toggle: boolean }) => void
  onActivate: (id: number) => void
  onRate: (id: number, rating: number) => void
}) {
  const { t } = useT()
  const [src, setSrc] = useState<string | null>(() => getThumb(photo.id))
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let active = true
    // Se já tem cache, mostra imediatamente e não re-busca.
    const cached = getThumb(photo.id)
    if (cached) {
      setSrc(cached)
      return () => {
        active = false
      }
    }
    setSrc(null)
    setFailed(false)
    window.openshoot
      .thumbForPhoto(photo.id, maxDim)
      .then((t) => {
        if (active) {
          if (t) {
            setSrc(t)
            setThumb(photo.id, t)
          } else setFailed(true)
        }
      })
      .catch(() => {
        if (active) setFailed(true)
      })
    return () => {
      active = false
    }
  }, [photo.id, maxDim])

  const label = photo.fileName

  const isPick = photo.rating >= 4
  const isReject = photo.rating >= 1 && photo.rating <= 2

  return (
    <div
      className={`cell ${selected ? 'selected' : ''} ${isPick ? 'flag-pick' : ''} ${
        isReject ? 'flag-reject' : ''
      }`}
      onClick={(e) =>
        onSelect(photo.id, { extend: e.shiftKey, toggle: e.metaKey || e.ctrlKey })
      }
      onDoubleClick={() => onActivate(photo.id)}
      title={photo.path}
    >
      <div className="cell-img">
        {src ? (
          <img src={src} alt={label} loading="lazy" draggable={false} />
        ) : failed ? (
          <div className="cell-fallback">{t('gallery.semPreview')}</div>
        ) : (
          <div className="cell-loading" />
        )}
        {isPick && <span className="cell-flag flag-green">P</span>}
        {isReject && <span className="cell-flag flag-red">X</span>}
        {photo.cullScore != null && (
          <span className="cell-score">{Math.round(photo.cullScore)}</span>
        )}
        {selected && <span className="cell-check">✓</span>}
        <div
          className="cell-stars"
          role="radiogroup"
          aria-label={t('gallery.starRating')}
          onClick={(e) => e.stopPropagation()}
          onDoubleClick={(e) => e.stopPropagation()}
        >
          {[1, 2, 3, 4, 5].map((star) => (
            <button
              key={star}
              type="button"
              role="radio"
              aria-checked={photo.rating === star}
              aria-label={t('gallery.starN', { n: star })}
              className={`cell-star ${star <= photo.rating ? 'on' : ''}`}
              onClick={(e) => {
                e.stopPropagation()
                onRate(photo.id, star === photo.rating ? 0 : star)
              }}
            >
              ★
            </button>
          ))}
        </div>
      </div>
      <div className="cell-meta">
        <span className="cell-name">{label}</span>
        {photo.camera && <span className="cell-camera">{photo.camera}</span>}
      </div>
    </div>
  )
}

function cellComponent({
  columnIndex,
  rowIndex,
  style,
  photos,
  cols,
  maxDim,
  selectedIds,
  onSelect,
  onActivate,
  onRate
}: {
  columnIndex: number
  rowIndex: number
  style: React.CSSProperties
  photos: PhotoMeta[]
  cols: number
  maxDim: number
  selectedIds: Set<number>
  onSelect: (id: number, opts: { extend: boolean; toggle: boolean }) => void
  onActivate: (id: number) => void
  onRate: (id: number, rating: number) => void
}) {
  const index = rowIndex * cols + columnIndex
  const photo = photos[index]
  if (!photo) return <div style={style} />
  return (
    <div style={style}>
      <Thumb
        photo={photo}
        maxDim={maxDim}
        selected={selectedIds.has(photo.id)}
        onSelect={onSelect}
        onActivate={onActivate}
        onRate={onRate}
      />
    </div>
  )
}

export default function Gallery({
  photos,
  onRefresh,
  selectedIds,
  anchorId,
  onSelect,
  onActivate,
  onRate
}: GalleryProps) {
  const { t } = useT()
  const [cols, setCols] = useState(() => computeCols(DEFAULT_WIDTH))
  const gridRef = useRef<any>(null)

  const colCount = cols
  const rowCount = Math.max(1, Math.ceil(photos.length / colCount))
  const cellProps: CellData = {
    photos,
    cols: colCount,
    maxDim: CELL_SIZE * 2,
    selectedIds,
    onSelect,
    onActivate,
    onRate
  }

  // Rola até uma foto quando o anchor muda por teclado.
  useEffect(() => {
    if (!anchorId) return
    const idx = photos.findIndex((p) => p.id === anchorId)
    if (idx < 0) return
    const row = Math.floor(idx / colCount)
    const col = idx % colCount
    gridRef.current?.scrollToCell?.({ columnIndex: col, rowIndex: row })
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [anchorId, colCount])

  return (
    <div className="gallery">
      <div className="gallery-bar">
        <span>
          {t('gallery.fotoCount', { n: photos.length })}
          {selectedIds.size > 0 &&
            t('gallery.selecionadasCount', { n: selectedIds.size })}
        </span>
        <button className="ghost" onClick={onRefresh}>
          {t('gallery.atualizar')}
        </button>
      </div>
      {photos.length === 0 ? (
        <div
          className="gallery-empty"
          dangerouslySetInnerHTML={{ __html: t('gallery.semFotos') }}
        />
      ) : (
        <div className="gallery-scroll">
          <Grid<CellData>
            gridRef={gridRef}
            className="openshoot-grid"
            cellComponent={cellComponent}
            cellProps={cellProps}
            columnCount={colCount}
            columnWidth={CELL_SIZE}
            rowCount={rowCount}
            rowHeight={ROW_HEIGHT}
            defaultWidth={DEFAULT_WIDTH}
            defaultHeight={600}
            overscanCount={3}
            onResize={({ width }) => setCols(computeCols(width))}
          />
        </div>
      )}
    </div>
  )
}
