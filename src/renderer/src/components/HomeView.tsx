import { useCallback, useEffect, useState } from 'react'
import { useT } from '../i18n/I18nContext'
import SettingsControl from './SettingsControl'

interface AlbumItem {
  id: number
  name: string
  sessionType: string
  coverPhotoId: number | null
  createdAt: string
  photoCount: number
  coverPath: string | null
}

interface HomeViewProps {
  onOpenAlbum: (albumId: number) => void
}

export default function HomeView({ onOpenAlbum }: HomeViewProps) {
  const { t } = useT()
  const [albums, setAlbums] = useState<AlbumItem[]>([])
  const [creating, setCreating] = useState(false)
  const [albumToDelete, setAlbumToDelete] = useState<AlbumItem | null>(null)
  const [name, setName] = useState('')
  const [covers, setCovers] = useState<Record<number, string>>({})

  const load = useCallback(() => {
    window.openshoot.listAlbums().then((a) => setAlbums(a))
  }, [])

  useEffect(() => {
    load()
  }, [load])

  // Carrega a capa de cada álbum (thumbnail do primeiro caminho).
  useEffect(() => {
    const coversToLoad = albums.filter((a) => a.coverPath && !covers[a.id])
    coversToLoad.forEach((a) => {
      window.openshoot
        .thumbForPath(a.coverPath!, 300)
        .then((t) => t && setCovers((c) => ({ ...c, [a.id]: t })))
        .catch(() => {})
    })
  }, [albums, covers])

  const create = useCallback(() => {
    const n = name.trim()
    if (!n) return
    window.openshoot.createAlbum(n).then((id) => {
      if (id > 0) {
        setCreating(false)
        setName('')
        load()
      }
    })
  }, [name, load])

  const del = useCallback(
    (album: AlbumItem) => {
      window.openshoot.deleteAlbum(album.id).then(() => {
        setAlbumToDelete(null)
        load()
      })
    },
    [load]
  )

  return (
    <div className="home">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <div className="topbar-right">
          <SettingsControl />
          <button onClick={() => setCreating(true)} className="primary">
            + {t('home.novoAlbum')}
          </button>
        </div>
      </header>

      {creating && (
        <div className="dialog-overlay">
          <div className="dialog">
            <h3>{t('home.criarAlbum')}</h3>
            <input
              className="album-name-input"
              type="text"
              autoFocus
              placeholder={t('home.nomeAlbum')}
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') create()
                if (e.key === 'Escape') setCreating(false)
              }}
            />
            <div className="import-actions">
              <button onClick={() => setCreating(false)} className="ghost">
                {t('dialog.cancel')}
              </button>
              <button onClick={create} disabled={!name.trim()} className="primary">
                {t('home.criar')}
              </button>
            </div>
          </div>
        </div>
      )}

      {albumToDelete && (
        <div className="dialog-overlay" role="presentation">
          <section className="dialog" role="dialog" aria-modal="true" aria-labelledby="delete-album-title">
            <h3 id="delete-album-title">Excluir “{albumToDelete.name}”?</h3>
            <p className="edit-hint">O álbum e sua organização serão removidos. As fotos originais e seus arquivos XMP não serão apagados.</p>
            <div className="import-actions">
              <button onClick={() => setAlbumToDelete(null)} className="ghost">Cancelar</button>
              <button onClick={() => del(albumToDelete)} className="danger">Excluir álbum</button>
            </div>
          </section>
        </div>
      )}

      <main className="home-body">
        {albums.length === 0 ? (
          <div className="home-empty">
            <div className="home-empty-title">{t('home.semAlbuns')}</div>
            <p>{t('home.semAlbunsHint')}</p>
            <button onClick={() => setCreating(true)} className="primary">
              + {t('home.novoAlbum')}
            </button>
          </div>
        ) : (
          <div className="home-grid">
            <div className="home-create-card" onClick={() => setCreating(true)}>
              <span className="home-create-plus">+</span>
              <span>{t('home.novoAlbum')}</span>
            </div>
            {albums.map((a) => (
              <div
                key={a.id}
                className="album-card"
                role="button"
                tabIndex={0}
                aria-label={`Abrir álbum ${a.name}`}
                onClick={() => onOpenAlbum(a.id)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault()
                    onOpenAlbum(a.id)
                  }
                }}
              >
                <div className="album-cover">
                  {covers[a.id] ? (
                    <img src={covers[a.id]} alt={a.name} />
                  ) : (
                    <div className="album-cover-empty">📷</div>
                  )}
                  <span className="album-count">{t('app.fotos', { n: a.photoCount })}</span>
                </div>
                <div className="album-meta">
                  <span className="album-name">{a.name}</span>
                  {a.sessionType && <span className="album-session">{a.sessionType}</span>}
                </div>
                <button
                  className="album-delete"
                  onClick={(e) => {
                    e.stopPropagation()
                    setAlbumToDelete(a)
                  }}
                  title={t('home.deletarAlbum')}
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  )
}
