import { useCallback, useEffect, useRef, useState, type CSSProperties } from 'react'
import { useT } from '../i18n/I18nContext'
import SettingsControl from './SettingsControl'
import WorkspaceNav, { type WorkspaceSection } from './WorkspaceNav'

interface PeopleViewProps {
  albumId: number
  activeSection: WorkspaceSection
  onBack: () => void
  onNavigate: (section: WorkspaceSection) => void
  onOpenEdit: (photoId: number) => void
  onOpenRetouch: (photoId: number) => void
}

interface EnrichedGroup {
  id: number
  name: string
  count: number
  cover?: string | null
  faces: Array<{ id: number; photo_id: number; bbox: [number, number, number, number]; group_name: string }>
}

interface FaceCrop {
  style: CSSProperties
  crop: { x: number; y: number; w: number; h: number } | null
  naturalW: number
  naturalH: number
}

function faceCrop(
  bbox: [number, number, number, number] | undefined,
  natural: { width: number; height: number } | null,
  frame: { width: number; height: number } | null
): FaceCrop {
  if (!bbox || !natural || !frame || frame.width <= 0 || frame.height <= 0) {
    return {
      style: { width: '100%', height: '100%', objectFit: 'cover' },
      crop: null,
      naturalW: natural?.width ?? 1,
      naturalH: natural?.height ?? 1
    }
  }
  const [x1, y1, x2, y2] = bbox
  const frameAspect = frame.width / frame.height
  const faceWidth = Math.max(1, (x2 - x1) * natural.width)
  const faceHeight = Math.max(1, (y2 - y1) * natural.height)
  // Reserva margem suficiente para mostrar o rosto inteiro, respeitando a
  // proporção do card e sem distorcer a imagem.
  const margin = 1.2
  let cropWidth = Math.max(faceWidth * margin, faceHeight * margin * frameAspect)
  let cropHeight = cropWidth / frameAspect
  if (cropHeight > natural.height) {
    cropHeight = natural.height
    cropWidth = cropHeight * frameAspect
  }
  if (cropWidth > natural.width) {
    cropWidth = natural.width
    cropHeight = cropWidth / frameAspect
  }
  const centerX = ((x1 + x2) / 2) * natural.width
  const centerY = ((y1 + y2) / 2) * natural.height
  const cropX = Math.min(Math.max(0, centerX - cropWidth / 2), natural.width - cropWidth)
  const cropY = Math.min(Math.max(0, centerY - cropHeight / 2), natural.height - cropHeight)
  const scale = frame.width / cropWidth
  return {
    style: {
      width: natural.width * scale,
      height: natural.height * scale,
      left: -cropX * scale,
      top: -cropY * scale,
      maxWidth: 'none',
      maxHeight: 'none'
    },
    crop: { x: cropX, y: cropY, w: cropWidth, h: cropHeight },
    naturalW: natural.width,
    naturalH: natural.height
  }
}

// Converte uma bbox normalizada (da foto original) para o espaço do crop
// (posição em % dentro do frame). Retorna null se estiver totalmente fora.
function bboxInCrop(
  bbox: [number, number, number, number],
  crop: { x: number; y: number; w: number; h: number },
  naturalW: number,
  naturalH: number
): CSSProperties | null {
  const pxX1 = bbox[0] * naturalW
  const pxY1 = bbox[1] * naturalH
  const pxX2 = bbox[2] * naturalW
  const pxY2 = bbox[3] * naturalH
  if (pxX2 < crop.x || pxY2 < crop.y || pxX1 > crop.x + crop.w || pxY1 > crop.y + crop.h) {
    return null
  }
  const left = ((pxX1 - crop.x) / crop.w) * 100
  const top = ((pxY1 - crop.y) / crop.h) * 100
  const width = ((pxX2 - pxX1) / crop.w) * 100
  const height = ((pxY2 - pxY1) / crop.h) * 100
  return { left: `${left}%`, top: `${top}%`, width: `${width}%`, height: `${height}%` }
}

function FaceThumbnail({ src, bbox, faces, alt, className }: {
  src?: string | null
  bbox?: [number, number, number, number]
  faces?: Array<[number, number, number, number]>
  alt: string
  className?: string
}) {
  if (!src) return <div className="person-cover-empty" aria-hidden="true">👤</div>
  const frameRef = useRef<HTMLDivElement | null>(null)
  const [natural, setNatural] = useState<{ width: number; height: number } | null>(null)
  const [frame, setFrame] = useState<{ width: number; height: number } | null>(null)
  useEffect(() => {
    const element = frameRef.current
    if (!element) return
    const update = () => setFrame({ width: element.clientWidth, height: element.clientHeight })
    update()
    const observer = new ResizeObserver(update)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])
  const cropInfo = faceCrop(bbox, natural, frame)
  const cropRect = cropInfo.crop
  return (
    <div ref={frameRef} className={`face-thumb-frame ${className ?? ''}`}>
      <img
        src={src}
        alt={alt}
        onLoad={(event) => setNatural({ width: event.currentTarget.naturalWidth, height: event.currentTarget.naturalHeight })}
        style={cropInfo.style}
      />
      {faces && faces.length > 0 && cropRect && (
        <div className="face-thumb-overlay">
          {faces.map((fb, i) => {
            const pos = bboxInCrop(fb, cropRect, cropInfo.naturalW, cropInfo.naturalH)
            if (!pos) return null
            return <span key={i} className="face-thumb-box" style={pos} />
          })}
        </div>
      )}
    </div>
  )
}

export default function PeopleView({ albumId, activeSection, onBack, onNavigate, onOpenEdit, onOpenRetouch }: PeopleViewProps) {
  const { t } = useT()
  const [threshold, setThreshold] = useState(0.5)
  const [grouping, setGrouping] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [groups, setGroups] = useState<EnrichedGroup[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [analysisNotice, setAnalysisNotice] = useState<string | null>(null)
  const [detailGroup, setDetailGroup] = useState<EnrichedGroup | null>(null)
  const [detailThumbs, setDetailThumbs] = useState<Record<number, string>>({})
  const [renamingId, setRenamingId] = useState<number | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const [renameError, setRenameError] = useState<string | null>(null)
  const renameRef = useRef<HTMLInputElement | null>(null)
  const [albumPhotoIds, setAlbumPhotoIds] = useState<number[] | null>(null)
  const [albumPhotoIdsError, setAlbumPhotoIdsError] = useState<string | null>(null)
  const [exportResult, setExportResult] = useState<{ n: number; dir: string } | null>(null)

  useEffect(() => {
    let active = true
    setAlbumPhotoIds(null)
    setAlbumPhotoIdsError(null)
    window.openshoot.albumPhotoIds(albumId).then((ids) => {
      if (active) setAlbumPhotoIds(ids)
    }).catch((e) => {
      if (active) setAlbumPhotoIdsError(String(e))
    })
    return () => { active = false }
  }, [albumId])

  const loadGroups = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const res = await window.openshoot.listPersonGroups(albumId)
      if (!res.ok || !res.groups) {
        setError(res.error ?? t('people.erroCarregar'))
        setGroups([])
        return
      }
      const enriched: EnrichedGroup[] = []
      for (const pg of res.groups) {
        const facesRes = await window.openshoot.listFacesInGroup(pg.id)
        const faces = facesRes.ok && facesRes.faces ? facesRes.faces : []
        const uniquePhotoIds = [...new Set(faces.map((f) => f.photo_id))]
        let cover: string | null = null
        if (faces.length > 0) {
          try {
            cover = await window.openshoot.thumbForPhoto(faces[0].photo_id, 500)
          } catch {
            cover = null
          }
        }
        enriched.push({ id: pg.id, name: pg.name, count: uniquePhotoIds.length, faces, cover })
      }
      setGroups(enriched)
      setDetailGroup((prev) => {
        if (!prev) return null
        const updated = enriched.find((g) => g.id === prev.id)
        return updated ?? prev
      })
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }, [albumId, t])

  useEffect(() => {
    loadGroups()
  }, [loadGroups])

  const albumPhotoIdsLoaded = albumPhotoIds !== null
  const albumEmpty = albumPhotoIdsLoaded && albumPhotoIds.length === 0

  const runGrouping = useCallback(async () => {
    if (!albumPhotoIdsLoaded || albumEmpty) return
    setGrouping(true)
    setError(null)
    setAnalysisNotice(null)
    try {
      const res = await window.openshoot.groupBySimilarity(threshold, albumPhotoIds, albumId)
      if ('error' in res) {
        setError(String(res.error))
        return
      }
      if (res.photos_unavailable > 0) {
        setAnalysisNotice(t('people.analysisPartial', {
          scanned: res.photos_scanned,
          unavailable: res.photos_unavailable,
        }))
      } else if (res.groups.length === 0) {
        setAnalysisNotice(t('people.analysisNoFaces', { n: res.photos_scanned }))
      }
      await loadGroups()
    } catch (e) {
      setError(String(e))
    } finally {
      setGrouping(false)
    }
  }, [albumId, albumPhotoIds, albumPhotoIdsLoaded, albumEmpty, threshold, loadGroups])

  const runExport = useCallback(async () => {
    if (!albumPhotoIdsLoaded || albumEmpty) return
    const outDir = await window.openshoot.pickExportFolder()
    if (!outDir) return
    setExporting(true)
    setError(null)
    try {
      const res = await window.openshoot.exportPersistedPeopleAlbum(albumId, outDir)
      if (!res.ok) {
        setError(res.error ?? 'erro')
      } else {
        setExportResult({ n: res.exported ?? 0, dir: res.out_dir ?? outDir })
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setExporting(false)
    }
  }, [albumId, albumPhotoIdsLoaded, albumEmpty])

  const startRename = useCallback((group: EnrichedGroup) => {
    setRenamingId(group.id)
    setRenameValue(group.name)
    setRenameError(null)
    setTimeout(() => renameRef.current?.focus(), 0)
  }, [])

  const commitRename = useCallback(async () => {
    if (renamingId == null) return
    const name = renameValue.trim()
    if (!name) {
      setRenameError(t('people.erroRenomear'))
      return
    }
    try {
      const res = await window.openshoot.renamePersonGroup(renamingId, name)
      if (!res.ok) {
        setRenameError(res.error ?? t('people.erroRenomear'))
        return
      }
      setGroups((prev) => prev.map((g) => g.id === renamingId ? { ...g, name } : g))
      setDetailGroup((prev) => prev && prev.id === renamingId ? { ...prev, name } : prev)
      setRenamingId(null)
    } catch (e) {
      setRenameError(String(e))
    }
  }, [renamingId, renameValue, t])

  const cancelRename = useCallback(() => {
    setRenamingId(null)
    setRenameError(null)
  }, [])

  useEffect(() => {
    if (detailGroup) {
      const ids = detailGroup.faces.map((f) => f.photo_id)
      const unique = [...new Set(ids)]
      const missing = unique.filter((id) => !detailThumbs[id])
      if (missing.length === 0) return
      let active = true
      Promise.all(missing.map(async (id) => {
        try {
          const thumb = await window.openshoot.thumbForPhoto(id, 300)
          return { id, thumb }
        } catch {
          return null
        }
      })).then((results) => {
        if (!active) return
        const next = { ...detailThumbs }
        for (const r of results) {
          if (r?.thumb) next[r.id] = r.thumb
        }
        setDetailThumbs(next)
      })
      return () => { active = false }
    }
  }, [detailGroup, detailThumbs])

  const handleOpenDetail = useCallback((group: EnrichedGroup) => {
    setDetailGroup(group)
  }, [])

  if (detailGroup) {
    const uniquePhotoIds = [...new Set(detailGroup.faces.map((f) => f.photo_id))]
    return (
      <div className="people">
        <header className="topbar workspace-topbar">
          <div className="workspace-left"><span className="logo">OpenShoot</span></div>
          <WorkspaceNav active={activeSection} onNavigate={onNavigate} />
          <div className="topbar-right workspace-actions">
            <SettingsControl />
          </div>
        </header>
        <div className="workspace-contextbar people-contextbar" aria-label={t('people.detalhes')}>
          <button onClick={() => setDetailGroup(null)} className="ghost back-albums">
            ← {t('people.titulo')}
          </button>
        </div>
        <main className="people-body">
          <div className="person-detail-header">
            <span className="person-detail-title">{detailGroup.name}</span>
            <span className="person-detail-count">{t('gallery.fotoCount', { n: uniquePhotoIds.length })}</span>
          </div>
          <div className="person-detail-grid">
            {uniquePhotoIds.map((photoId) => (
              <div key={photoId} className="person-detail-item">
                  <div className="person-detail-thumb">
                    <FaceThumbnail
                      src={detailThumbs[photoId]}
                      bbox={detailGroup.faces.find((face) => face.photo_id === photoId)?.bbox}
                      alt={t('people.faceCropAlt', { name: detailGroup.name })}
                      className="person-face-crop"
                    />
                  </div>
                  <div className="person-detail-actions">
                    <button
                      onClick={() => onOpenEdit(photoId)}
                      className="ghost small"
                      title={t('people.abrirEditarDetalhe')}
                    >
                      {t('people.abrirEditarDetalhe')}
                    </button>
                    <button
                      onClick={() => onOpenRetouch(photoId)}
                      className="ghost small"
                      title={t('people.abrirRetoqueDetalhe')}
                    >
                      {t('people.abrirRetoqueDetalhe')}
                    </button>
                  </div>
                </div>
            ))}
          </div>
          {uniquePhotoIds.length === 0 && (
            <div className="people-empty">
              <div className="people-empty-card">
                <div className="people-empty-icon" aria-hidden="true">◉</div>
                <p>{t('people.semNenhum')}</p>
              </div>
            </div>
          )}
        </main>
      </div>
    )
  }

  return (
    <div className="people">
      <header className="topbar workspace-topbar">
        <div className="workspace-left"><span className="logo">OpenShoot</span></div>
        <WorkspaceNav active={activeSection} onNavigate={onNavigate} />
        <div className="topbar-right workspace-actions">
          <SettingsControl />
        </div>
      </header>
      <div className="workspace-contextbar people-contextbar" aria-label="Ferramentas de pessoas">
        <label className="people-threshold">
          <span>{t('people.similaridade')}</span>
          <input
            type="range"
            min={0.3}
            max={0.8}
            step={0.01}
            value={threshold}
            onChange={(e) => setThreshold(Number(e.target.value))}
            title={`${t('people.similaridadeMin')} ↔ ${t('people.similaridadeMax')}`}
          />
          <em>{threshold.toFixed(2)}</em>
          <span className="people-threshold-hint">
            {t('people.similaridadeMin')} ↔ {t('people.similaridadeMax')}
          </span>
        </label>
        <button
          onClick={runGrouping}
          disabled={grouping || exporting || !albumPhotoIdsLoaded || albumEmpty}
        >
          {grouping ? t('people.agrupando') : t('people.identifyPeople')}
        </button>
        <button
          onClick={runExport}
          disabled={grouping || exporting || groups.length === 0 || !albumPhotoIdsLoaded || albumEmpty}
        >
          {exporting ? '…' : t('people.exportar')}
        </button>
        <button onClick={onBack} className="ghost back-albums">
          ← {t('people.voltar')}
        </button>
      </div>

      {error && <div className="toast error">{error}</div>}

      {analysisNotice && <div className="toast info" role="status">{analysisNotice}</div>}

      {exportResult && (
        <div className="toast success">
          {t('people.exportSuccess', { n: exportResult.n, dir: exportResult.dir })}
          <button onClick={() => setExportResult(null)} className="ghost small">✕</button>
        </div>
      )}

      {!albumPhotoIdsLoaded && !albumPhotoIdsError && (
        <div className="people-loading">
          <span className="people-spinner" />
          {t('people.loadingAlbum')}
        </div>
      )}

      {albumPhotoIdsError && (
        <div className="toast error">{albumPhotoIdsError}</div>
      )}

      {albumPhotoIdsLoaded && albumEmpty && (
        <div className="people-empty">
          <div className="people-empty-card">
            <div className="people-empty-icon" aria-hidden="true">◉</div>
            <p>{t('people.noPhotosInAlbum')}</p>
          </div>
        </div>
      )}

      {albumPhotoIdsLoaded && !albumEmpty && (
        <main className="people-body">
          {loading || grouping ? (
            <div className="people-loading">
              <span className="people-spinner" />
              {grouping ? t('people.identifyingPeople') : t('people.loadingAlbum')}
            </div>
          ) : groups.length === 0 ? (
            <div className="people-empty">
              <div className="people-empty-card">
                <div className="people-empty-icon" aria-hidden="true">◉</div>
                <div className="home-empty-title">{t('people.titulo')}</div>
                <p>{t('people.analisar')}</p>
              </div>
            </div>
          ) : (
            <>
              <div className="people-intro">
                <div>
                  <h1>{t('people.titulo')}</h1>
                  <p>{t('people.reviewIntro')}</p>
                </div>
                <div className="people-reanalyze-hint">
                  {t('people.reanalyzeWarning')}
                </div>
              </div>
              <div className="people-grid">
                {groups.map((g) => (
                  <article
                    key={g.id}
                    className="person-card"
                  >
                    <button
                      type="button"
                      className="person-open"
                      onClick={() => handleOpenDetail(g)}
                      aria-label={t('people.openGroup', { name: g.name })}
                    >
                      <div className="person-cover">
                        <FaceThumbnail
                          src={g.cover}
                          bbox={g.faces[0]?.bbox}
                          faces={g.faces.map((f) => f.bbox)}
                          alt={t('people.faceCropAlt', { name: g.name })}
                          className="person-face-crop"
                        />
                        <span className="person-count">{t('gallery.fotoCount', { n: g.count })}</span>
                      </div>
                    </button>
                    <div className="person-meta">
                      {renamingId === g.id ? (
                        <div className="person-rename-row">
                          <input
                            ref={renameRef}
                            type="text"
                            className="person-rename-input"
                            value={renameValue}
                            onChange={(e) => { setRenameValue(e.target.value); setRenameError(null) }}
                            onKeyDown={(e) => {
                              if (e.key === 'Enter') commitRename()
                              if (e.key === 'Escape') cancelRename()
                            }}
                            onBlur={commitRename}
                            aria-label={t('people.renomear')}
                          />
                          {renameError && <span className="person-rename-error" role="alert">{renameError}</span>}
                        </div>
                      ) : (
                        <button
                          type="button"
                          className="person-name"
                          title={t('people.confirmName')}
                          onClick={() => startRename(g)}
                        >
                          <span>{g.name}</span>
                          <span className="person-name-action">{t('people.confirmName')}</span>
                        </button>
                      )}
                      <p className="person-review-hint">{t('people.openToReview')}</p>
                    </div>
                  </article>
                ))}
              </div>
            </>
          )}
        </main>
      )}
    </div>
  )
}
