import { useEffect, useState } from 'react'
import { Grid } from 'react-window'
import type { PhotoMeta } from '../../../types/photo'

interface GalleryProps {
  photos: PhotoMeta[]
  onRefresh: () => void
  selectedId: number | null
  onSelect: (id: number) => void
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
  selectedId: number | null
  onSelect: (id: number) => void
}

function Thumb({
  photo,
  maxDim,
  selected,
  onSelect
}: {
  photo: PhotoMeta
  maxDim: number
  selected: boolean
  onSelect: (id: number) => void
}) {
  const [src, setSrc] = useState<string | null>(null)
  const [failed, setFailed] = useState(false)

  useEffect(() => {
    let active = true
    setSrc(null)
    setFailed(false)
    window.openshoot
      .thumbForPhoto(photo.id, maxDim)
      .then((t) => {
        if (active) {
          if (t) setSrc(t)
          else setFailed(true)
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

  return (
    <div
      className={`cell ${selected ? 'selected' : ''}`}
      onClick={() => onSelect(photo.id)}
      title={photo.path}
    >
      <div className="cell-img">
        {src ? (
          <img src={src} alt={label} loading="lazy" draggable={false} />
        ) : failed ? (
          <div className="cell-fallback">sem preview</div>
        ) : (
          <div className="cell-loading" />
        )}
        {photo.rating > 0 && <span className="cell-rating">★{photo.rating}</span>}
        {photo.cullScore != null && (
          <span className="cell-score">{Math.round(photo.cullScore)}</span>
        )}
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
  selectedId,
  onSelect
}: {
  columnIndex: number
  rowIndex: number
  style: React.CSSProperties
  photos: PhotoMeta[]
  cols: number
  maxDim: number
  selectedId: number | null
  onSelect: (id: number) => void
}) {
  const index = rowIndex * cols + columnIndex
  const photo = photos[index]
  if (!photo) return <div style={style} />
  return (
    <div style={style}>
      <Thumb
        photo={photo}
        maxDim={maxDim}
        selected={selectedId === photo.id}
        onSelect={onSelect}
      />
    </div>
  )
}

export default function Gallery({ photos, onRefresh, selectedId, onSelect }: GalleryProps) {
  const [cols, setCols] = useState(() => computeCols(DEFAULT_WIDTH))

  const colCount = cols
  const rowCount = Math.max(1, Math.ceil(photos.length / colCount))
  const cellProps: CellData = {
    photos,
    cols: colCount,
    maxDim: CELL_SIZE * 2,
    selectedId,
    onSelect
  }

  return (
    <div className="gallery">
      <div className="gallery-bar">
        <span>
          {photos.length} foto{photos.length === 1 ? '' : 's'}
        </span>
        <button className="ghost" onClick={onRefresh}>
          Atualizar
        </button>
      </div>
      {photos.length === 0 ? (
        <div className="gallery-empty">
          Nenhuma foto importada. Clique em <strong>Importar pasta</strong> para começar.
        </div>
      ) : (
        <div className="gallery-scroll">
          <Grid<CellData>
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
