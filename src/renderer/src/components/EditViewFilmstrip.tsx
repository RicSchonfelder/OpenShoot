import { useEffect, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'

interface EditViewFilmstripProps {
  photos: PhotoMeta[]
  activeId: number
  onSelect: (id: number) => void
}

export default function EditViewFilmstrip({ photos, activeId, onSelect }: EditViewFilmstripProps) {
  const [thumbs, setThumbs] = useState<Record<number, string>>({})

  useEffect(() => {
    let active = true
    Promise.all(photos.map(async (photo) => {
      try {
        const thumb = await window.openshoot.thumbForPhoto(photo.id, 180)
        return thumb ? ([photo.id, thumb] as const) : null
      } catch {
        return null
      }
    })).then((entries) => {
      if (active) setThumbs(Object.fromEntries(entries.filter((entry): entry is readonly [number, string] => Boolean(entry))))
    })
    return () => { active = false }
  }, [photos])

  return (
    <div
      className="editview-filmstrip"
      aria-label="Fotos do álbum"
      onWheel={(event) => {
        if (event.deltaY === 0) return
        event.preventDefault()
        event.currentTarget.scrollLeft += event.deltaY
      }}
    >
      {photos.map((photo) => (
        <button
          key={photo.id}
          className={`editview-filmstrip-item ${photo.id === activeId ? 'active' : ''}`}
          onClick={() => onSelect(photo.id)}
          title={photo.fileName}
        >
          {thumbs[photo.id] ? <img src={thumbs[photo.id]} alt="" /> : <span>Carregando…</span>}
          <small>{photo.fileName}</small>
        </button>
      ))}
    </div>
  )
}
