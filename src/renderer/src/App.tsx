import { useCallback, useEffect, useRef, useState } from 'react'
import Gallery from './components/Gallery'
import EditPanel from './components/EditPanel'
import LoupeView from './components/LoupeView'
import FilterPanel from './components/FilterPanel'
import { useT } from './i18n/I18nContext'
import type { PhotoMeta } from '../../types/photo'

const PAGE_SIZE = 1000

type Filter =
  | 'all'
  | 'picks'
  | 'rejects'
  | 'unrated'
  | 'duplicates'
  | 'faces'
  | 'review'
  | 'portrait'
  | 'landscape'
  | 'raw'
  | 'jpeg'
  | 'destaques'
  | 'selecionado'
  | 'edited'
  | 'unedited'

type DeleteDialogState = 'none' | 'catalog' | 'trash'

const SESSION_TYPES = [
  { id: 'wedding', labelKey: 'import.wedding' },
  { id: 'portrait', labelKey: 'import.portrait' },
  { id: 'family', labelKey: 'import.family' },
  { id: 'school', labelKey: 'import.school' },
  { id: 'newborn', labelKey: 'import.newborn' },
  { id: 'sports', labelKey: 'import.sports' },
  { id: 'event', labelKey: 'import.event' },
  { id: 'boudoir', labelKey: 'import.boudoir' },
  { id: 'other', labelKey: 'import.other' }
]

export default function App() {
  const { t } = useT()
  const [photos, setPhotos] = useState<PhotoMeta[]>([])
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set())
  const [anchorId, setAnchorId] = useState<number | null>(null)
  const [filter, setFilter] = useState<Filter>('all')
  const [scanning, setScanning] = useState(false)
  const [scanProgress, setScanProgress] = useState<{ processed: number; total: number } | null>(null)
  const [culling, setCulling] = useState(false)
  const [exporting, setExporting] = useState(false)
  const [scanMsg, setScanMsg] = useState<string | null>(null)
  const [scanErrors, setScanErrors] = useState<string[]>([])
  const [error, setError] = useState<string | null>(null)
  const [loupeOpen, setLoupeOpen] = useState(false)
  const [loupeIndex, setLoupeIndex] = useState(0)
  const [deleteDialog, setDeleteDialog] = useState<DeleteDialogState>('none')
  const [moreOpen, setMoreOpen] = useState(false)
  const moreRef = useRef<HTMLDivElement | null>(null)
  const [targetPicks, setTargetPicks] = useState(0)
  const [pendingImport, setPendingImport] = useState<{
    dir: string
    includeSubdirs: boolean
    types: string
    sessionType: string
  } | null>(null)
  const photosRef = useRef<PhotoMeta[]>([])
  const selectedRef = useRef<Set<number>>(new Set())
  const anchorRef = useRef<number | null>(null)

  photosRef.current = photos
  selectedRef.current = selectedIds
  anchorRef.current = anchorId

  const loadPhotos = useCallback(
    async (f: Filter = filter) => {
      try {
        const list = await window.openshoot.listPhotos('', f, 0, PAGE_SIZE)
        setPhotos(list.photos)
        setSelectedIds(new Set())
        setAnchorId(null)
      } catch (e) {
        setError(String(e))
      }
    },
    [filter]
  )

  useEffect(() => {
    loadPhotos()
  }, [loadPhotos, filter])

  const selectedPhoto = photos.find((p) => p.id === anchorId) ?? null

  // ---- Rating manual com persistência XMP ----
  const persistRating = useCallback(
    async (ids: number[], rating: number) => {
      for (const id of ids) {
        try {
          await window.openshoot.setRating(id, rating)
          await window.openshoot.writeXmpForPhoto(id)
        } catch {
          // ignora falha de XMP (ex.: arquivo sem permissão de escrita)
        }
      }
    },
    []
  )

  // ---- Seleção múltipla ----
  const handleSelect = useCallback(
    (id: number, opts: { extend: boolean; toggle: boolean }) => {
      const idx = photosRef.current.findIndex((p) => p.id === id)
      if (idx < 0) return

      if (opts.extend && anchorRef.current != null) {
        // Shift: seleciona o intervalo do anchor até este
        const anchorIdx = photosRef.current.findIndex((p) => p.id === anchorRef.current)
        if (anchorIdx < 0) return
        const [lo, hi] = anchorIdx < idx ? [anchorIdx, idx] : [idx, anchorIdx]
        const next = new Set<number>()
        for (let i = lo; i <= hi; i++) next.add(photosRef.current[i].id)
        setSelectedIds(next)
      } else if (opts.toggle) {
        // Cmd/Ctrl: alterna este item mantendo os outros
        const next = new Set(selectedRef.current)
        if (next.has(id)) next.delete(id)
        else next.add(id)
        setSelectedIds(next)
      } else {
        // Clique simples: seleciona só este
        setSelectedIds(new Set([id]))
      }
      setAnchorId(id)
    },
    []
  )

  const handleActivate = useCallback((id: number) => {
    const idx = photosRef.current.findIndex((p) => p.id === id)
    if (idx >= 0) {
      setLoupeIndex(idx)
      setLoupeOpen(true)
    }
    setAnchorId(id)
    setSelectedIds(new Set([id]))
  }, [])

  // ---- Loupe ----
  const loupeNavigate = useCallback((index: number) => {
    setLoupeIndex(index)
    setAnchorId(photosRef.current[index]?.id ?? null)
  }, [])

  const loupeApplyRating = useCallback(
    async (rating: number, advance: boolean) => {
      const photo = photosRef.current[loupeIndex]
      if (!photo) return
      await persistRating([photo.id], rating)
      const updated = await window.openshoot.listPhotos('', filter, 0, PAGE_SIZE)
      setPhotos(updated.photos)
      if (advance && loupeIndex < updated.photos.length - 1) {
        setLoupeIndex(loupeIndex + 1)
      } else {
        setAnchorId(photo.id)
      }
    },
    [loupeIndex, filter, persistRating]
  )

  const loupeClose = useCallback(() => {
    setLoupeOpen(false)
    setAnchorId(null)
    setSelectedIds(new Set())
  }, [])

  // ---- Deletar (diálogo com 3 opções) ----
  const deleteSelected = useCallback(() => {
    if (selectedRef.current.size > 0) setDeleteDialog('catalog')
  }, [])

  const runDeleteAction = useCallback(
    async (mode: 'catalog' | 'trash') => {
      const ids = Array.from(selectedRef.current)
      if (ids.length === 0) return
      let done = 0
      for (const id of ids) {
        const r =
          mode === 'trash'
            ? await window.openshoot.deletePhoto(id)
            : await window.openshoot.removePhotoFromCatalog(id)
        if (r) done++
      }
      setScanMsg(
        mode === 'trash' ? t('app.lixeiraMsg', { n: done }) : t('app.removidasMsg', { n: done })
      )
      setSelectedIds(new Set())
      setAnchorId(null)
      setDeleteDialog('none')
      await loadPhotos()
    },
    [loadPhotos, t]
  )

  // ---- Rating via atalho ----
  const applyRating = useCallback(
    async (rating: number) => {
      const ids = Array.from(selectedRef.current)
      if (ids.length === 0) return
      await persistRating(ids, rating)
      await loadPhotos()
    },
    [persistRating, loadPhotos]
  )

  // ---- Rating via clique nas estrelas do grid ----
  const handleRate = useCallback(
    async (id: number, rating: number) => {
      await persistRating([id], rating)
      await loadPhotos()
    },
    [persistRating, loadPhotos]
  )

  // ---- Atalhos de teclado globais ----
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      // Não intercepta quando digitando em input
      const tag = (e.target as HTMLElement)?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return

      const photos = photosRef.current
      const sel = selectedRef.current
      const anchor = anchorRef.current

      // Cmd+A / Ctrl+A = selecionar todas as fotos
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'a') {
        e.preventDefault()
        if (photos.length > 0) {
          setSelectedIds(new Set(photos.map((p) => p.id)))
          setAnchorId(photos[0]?.id ?? null)
        }
        return
      }

      // Navegação por setas (na seleção)
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
        e.preventDefault()
        const idx = anchor != null ? photos.findIndex((p) => p.id === anchor) : -1
        const cur = idx >= 0 ? idx : 0
        let next = cur
        const cols = Math.max(2, Math.floor((window.innerWidth || 1200) / 200))
        if (e.key === 'ArrowRight') next = Math.min(photos.length - 1, cur + 1)
        else if (e.key === 'ArrowLeft') next = Math.max(0, cur - 1)
        else if (e.key === 'ArrowDown') next = Math.min(photos.length - 1, cur + cols)
        else if (e.key === 'ArrowUp') next = Math.max(0, cur - cols)
        const p = photos[next]
        if (p) {
          const extend = e.shiftKey
          handleSelect(p.id, { extend, toggle: false })
        }
        return
      }

      // P = pick (★5), X = reject (★1), U = sem flag (0)
      if (e.key === 'p' || e.key === 'P') { applyRating(5); return }
      if (e.key === 'x' || e.key === 'X') { applyRating(1); return }
      if (e.key === 'u' || e.key === 'U') { applyRating(0); return }
      // 1-5 = rating direto
      if (['1', '2', '3', '4', '5'].includes(e.key)) {
        applyRating(Number(e.key)); return
      }
      // Enter / Espaço = abre loupe no anchor
      if (e.key === 'Enter' || e.key === ' ') {
        if (anchor != null && sel.size > 0) {
          e.preventDefault()
          const idx = photos.findIndex((p) => p.id === anchor)
          if (idx >= 0) {
            setLoupeIndex(idx)
            setLoupeOpen(true)
          }
        }
        return
      }
      // Del = deletar (diálogo 3 opções)
      if (e.key === 'Delete' || e.key === 'Backspace') {
        if (sel.size > 0) { e.preventDefault(); deleteSelected(); return }
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [applyRating, deleteSelected, handleSelect])

  const importFolder = useCallback(async () => {
    setError(null)
    setScanMsg(null)
    setScanErrors([])
    const dir = await window.openshoot.pickFolder()
    if (!dir) return
    // Passo 2: modal de opções de importação.
    setPendingImport({ dir, includeSubdirs: true, types: 'all', sessionType: '' })
  }, [])

  const confirmImport = useCallback(
    async (opts: { dir: string; includeSubdirs: boolean; types: string; sessionType: string }) => {
      setPendingImport(null)
      setScanning(true)
      setScanProgress({ processed: 0, total: 0 })
      try {
        const res = await window.openshoot.scanFolderProgress(
          opts.dir,
          opts.includeSubdirs,
          opts.types,
          (p) => {
            setScanProgress({ processed: p.processed, total: p.total })
          }
        )
        let r
        try {
          r = JSON.parse(res)
        } catch {
          r = { scanned: 0, added: 0, updated: 0, errors: 0 }
        }
        // Registra o tipo de sessão (gênero) nas fotos recém-importadas.
        if (opts.sessionType) {
          await window.openshoot.setSessionType(opts.dir, opts.sessionType)
        }
        setScanMsg(
          t('app.scanMsg', {
            dir: opts.dir,
            scanned: r.scanned,
            added: r.added,
            updated: r.updated,
            errors: r.errors
          })
        )
        await loadPhotos()
      } catch (e) {
        setError(String(e))
      } finally {
        setScanning(false)
        setScanProgress(null)
      }
    },
    [loadPhotos, t]
  )

  const runCull = useCallback(async () => {
    setError(null)
    setCulling(true)
    try {
      const res = await window.openshoot.cullPhotos(targetPicks > 0 ? targetPicks : undefined)
      if ('error' in res) {
        setError(String(res.error))
      } else {
        setScanMsg(
          t('app.cullMsg', {
            processed: res.processed,
            picks: res.picks,
            avg: res.avgScore.toFixed(1)
          })
        )
        await loadPhotos()
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setCulling(false)
    }
  }, [loadPhotos, t, targetPicks])

  const runOneClick = useCallback(async () => {
    setError(null)
    setScanMsg(null)
    setCulling(true)
    try {
      // 1) Cull com meta (se definida) — seleção automática.
      const res = await window.openshoot.cullPhotos(targetPicks > 0 ? targetPicks : undefined)
      if ('error' in res) {
        setError(String(res.error))
        return
      }
      // 2) Aplica um preset (se houver um selecionado).
      const presets = await window.openshoot.listPresets()
      if (presets.length > 0) {
        const applied = await window.openshoot.applyEditAll(presets[0].recipe)
        let n = 0
        try {
          n = JSON.parse(applied).applied ?? 0
        } catch {
          /* ignore */
        }
        setScanMsg(
          t('app.oneClickMsg', { picks: res.picks, editadas: n, preset: presets[0].name })
        )
      } else {
        setScanMsg(t('app.oneClickMsgSemPreset', { picks: res.picks }))
      }
      await loadPhotos()
    } catch (e) {
      setError(String(e))
    } finally {
      setCulling(false)
    }
  }, [loadPhotos, t, targetPicks])

  const learnProfile = useCallback(async () => {
    setError(null)
    setCulling(true)
    try {
      const res = await window.openshoot.learnProfile()
      if (!res.ok) {
        setError(res.error ?? 'erro')
      } else {
        setScanMsg(t('app.learnMsg', { name: res.name ?? '', n: res.photos ?? 0 }))
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setCulling(false)
    }
  }, [t])

  const exportXmp = useCallback(async () => {
    setError(null)
    setExporting(true)
    try {
      const res = await window.openshoot.exportAllXmp()
      if ('error' in res) {
        setError(String(res.error))
      } else {
        setScanMsg(
          res.errors > 0
            ? t('app.xmpMsgErr', { exported: res.exported, errors: res.errors })
            : t('app.xmpMsg', { exported: res.exported })
        )
      }
    } catch (e) {
      setError(String(e))
    } finally {
      setExporting(false)
    }
  }, [t])

  const handleApplyAll = useCallback(
    async (json: string) => {
      try {
        const res = await window.openshoot.applyEditAll(json)
        let r
        try {
          r = JSON.parse(res)
        } catch {
          r = { applied: 0 }
        }
        setScanMsg(t('app.editMsg', { applied: r.applied }))
      } catch (e) {
        setError(String(e))
      }
    },
    [t]
  )

  const clearCache = useCallback(async () => {
    const n = await window.openshoot.clearThumbCache()
    setScanMsg(t('app.cacheMsg', { n }))
  }, [t])

  // ---- Contadores para toolbar ----
  const picksCount = photos.filter((p) => p.rating >= 4).length
  const rejectsCount = photos.filter((p) => p.rating >= 1 && p.rating <= 2).length
  const unratedCount = photos.filter((p) => p.rating === 0).length
  const reviewCount = photos.filter((p) => p.review).length
  const destaquesCount = photos.filter((p) => p.aiPick).length
  const selecionadoCount = photos.filter((p) => p.rating >= 4 && !p.aiPick).length

  const FILTER_LABELS: Record<Filter, string> = {
    all: t('app.todos'),
    picks: t('app.picks'),
    rejects: t('app.rejects'),
    unrated: t('app.unrated'),
    duplicates: t('app.duplicatas'),
    faces: t('app.comRosto'),
    review: t('app.paraRevisao'),
    portrait: t('app.retrato'),
    landscape: t('app.paisagem'),
    raw: t('app.raw'),
    jpeg: t('app.jpeg'),
    destaques: t('app.destaques'),
    selecionado: t('app.selecionado'),
    edited: t('app.editadas'),
    unedited: t('app.naoEditadas')
  }

  const MAIN_FILTERS: Filter[] = ['all', 'picks', 'rejects', 'unrated']

  // Fecha o dropdown "Outros" ao clicar fora.
  useEffect(() => {
    if (!moreOpen) return
    const onDoc = (e: MouseEvent) => {
      if (moreRef.current && !moreRef.current.contains(e.target as Node)) setMoreOpen(false)
    }
    document.addEventListener('mousedown', onDoc)
    return () => document.removeEventListener('mousedown', onDoc)
  }, [moreOpen])

  if (loupeOpen) {
    return (
      <LoupeView
        photos={photos}
        currentIndex={loupeIndex}
        onNavigate={loupeNavigate}
        onApplyRating={loupeApplyRating}
        onClose={loupeClose}
      />
    )
  }

  return (
    <div className="app">
      <header className="topbar">
        <span className="logo">OpenShoot</span>
        <div className="topbar-filters">
          {MAIN_FILTERS.map((f) => (
            <button
              key={f}
              className={`filter-btn ${filter === f ? 'active' : ''}`}
              onClick={() => setFilter(f)}
            >
              {FILTER_LABELS[f]}
            </button>
          ))}
        </div>
        <div className="topbar-right">
          {selectedIds.size > 0 && (
            <button onClick={deleteSelected} className="danger">
              {t('app.deletar')} ({selectedIds.size})
            </button>
          )}
          <button onClick={exportXmp} disabled={exporting || photos.length === 0} className="ghost">
            {exporting ? t('app.exportando') : t('app.exportarXmp')}
          </button>
          <button
            onClick={runOneClick}
            disabled={culling || photos.length === 0}
            className="ghost"
            title={t('app.oneClickHint')}
          >
            {culling ? t('app.importando') : t('app.oneClick')}
          </button>
          <button onClick={runCull} disabled={culling || photos.length === 0} className="primary">
            {culling ? t('app.cullRun') : t('app.cull')}
          </button>
          <button onClick={importFolder} disabled={scanning}>
            {scanning ? t('app.importando') : t('app.importar')}
          </button>
        </div>
      </header>

      {scanMsg && <div className="toast">{scanMsg}</div>}
      {scanProgress && (
        <div className="toast progress">
          {t('app.importProgress', {
            processed: scanProgress.processed,
            total: scanProgress.total || '…'
          })}
          {scanProgress.total > 0 && (
            <div className="progress-bar">
              <div
                className="progress-fill"
                style={{
                  width: `${Math.round((scanProgress.processed / scanProgress.total) * 100)}%`
                }}
              />
            </div>
          )}
        </div>
      )}
      {scanErrors.length > 0 && (
        <details className="scan-errors">
          <summary>{t('app.scanErrors', { n: scanErrors.length })}</summary>
          <ul>
            {scanErrors.map((er, i) => (
              <li key={i}>{er}</li>
            ))}
          </ul>
        </details>
      )}
      {error && <div className="toast error">{error}</div>}

      <div className="cull-toolbar">
        <span className={`toolbar-count picks ${filter === 'picks' ? 'active' : ''}`}>
          <b>P</b> {picksCount}
        </span>
        <span className={`toolbar-count rejects ${filter === 'rejects' ? 'active' : ''}`}>
          <b>X</b> {rejectsCount}
        </span>
        <span className={`toolbar-count review ${filter === 'review' ? 'active' : ''}`}>
          <b>?</b> {reviewCount}
        </span>
        <span className={`toolbar-count destaques ${filter === 'destaques' ? 'active' : ''}`}>
          <b>★IA</b> {destaquesCount}
        </span>
        <span className={`toolbar-count selecionado ${filter === 'selecionado' ? 'active' : ''}`}>
          <b>✔</b> {selecionadoCount}
        </span>
        <span className={`toolbar-count unrated ${filter === 'unrated' ? 'active' : ''}`}>
          <b>U</b> {unratedCount}
        </span>
        <span className="toolbar-count total">
          {t('app.fotos', { n: photos.length })}
        </span>
        <label className="target-picks">
          {t('app.targetPicks')}
          <input
            type="range"
            min={0}
            max={Math.max(1, photos.length)}
            step={1}
            value={targetPicks}
            onChange={(e) => setTargetPicks(Number(e.target.value))}
          />
          <em>{targetPicks === 0 ? '∞' : targetPicks}</em>
        </label>
        <button
          className="toolbar-select-all"
          onClick={() => {
            if (photos.length > 0) {
              setSelectedIds(new Set(photos.map((p) => p.id)))
              setAnchorId(photos[0]?.id ?? null)
            }
          }}
        >
          {t('app.selectAll')} (⌘A)
        </button>
        <button
          className="toolbar-learn"
          onClick={learnProfile}
          disabled={culling}
          title={t('app.learnHint')}
        >
          {t('app.learn')}
        </button>
        <div className="more-menu" ref={moreRef}>
          <button
            className={`more-trigger ${filter === 'duplicates' || filter === 'faces' ? 'active' : ''}`}
            onClick={() => setMoreOpen((v) => !v)}
          >
            {t('app.outros')} ▾
          </button>
          {moreOpen && (
            <div className="more-dropdown">
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosSelecoesIA')}</span>
                <button
                  className={`${filter === 'destaques' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('destaques')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.destaques')} ({destaquesCount})
                </button>
                <button
                  className={`${filter === 'selecionado' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('selecionado')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.selecionado')} ({selecionadoCount})
                </button>
              </div>
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosDuplicatas')}</span>
                <button
                  className={`${filter === 'duplicates' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('duplicates')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.duplicatas')}
                </button>
              </div>
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosRosto')}</span>
                <button
                  className={`${filter === 'faces' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('faces')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.comRosto')}
                </button>
              </div>
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosRevisao')}</span>
                <button
                  className={`${filter === 'review' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('review')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.paraRevisao')}
                </button>
              </div>
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosOrientacao')}</span>
                <button
                  className={`${filter === 'portrait' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('portrait')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.retrato')}
                </button>
                <button
                  className={`${filter === 'landscape' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('landscape')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.paisagem')}
                </button>
              </div>
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosTipoArquivo')}</span>
                <button
                  className={`${filter === 'raw' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('raw')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.raw')}
                </button>
                <button
                  className={`${filter === 'jpeg' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('jpeg')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.jpeg')}
                </button>
              </div>
              <div className="more-section">
                <span className="more-section-title">{t('app.outrosStatus')}</span>
                <button
                  className={`${filter === 'edited' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('edited')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.editadas')}
                </button>
                <button
                  className={`${filter === 'unedited' ? 'active' : ''}`}
                  onClick={() => {
                    setFilter('unedited')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.naoEditadas')}
                </button>
              </div>
              <div className="more-section more-reset">
                <button
                  className="more-reset-btn"
                  onClick={() => {
                    setFilter('all')
                    setMoreOpen(false)
                  }}
                >
                  {t('app.reiniciar')}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      <main className="content">
        <div className="main-gallery">
          <FilterPanel
            active={filter}
            onSelect={(f) => setFilter(f as Filter)}
          />
          <Gallery
            photos={photos}
            onRefresh={loadPhotos}
            selectedIds={selectedIds}
            anchorId={anchorId}
            onSelect={handleSelect}
            onActivate={handleActivate}
            onRate={handleRate}
          />
        </div>
        <EditPanel photo={selectedPhoto} onApplyAll={handleApplyAll} />
      </main>

      <div className="shortcuts-bar">
        <span dangerouslySetInnerHTML={{ __html: t('app.shortcuts') }} />
        <button className="cache-clear" onClick={clearCache}>
          {t('app.limparCache')}
        </button>
      </div>

      {deleteDialog !== 'none' && (
        <div className="dialog-overlay">
          <div className="dialog">
            <h3>{t('dialog.deleteTitle', { n: selectedIds.size })}</h3>
            <div className="dialog-actions">
              <button onClick={() => runDeleteAction('catalog')} className="dialog-catalog">
                <strong>{t('dialog.deleteRemoveCatalog')}</strong>
                <span>{t('dialog.deleteRemoveCatalogDesc')}</span>
              </button>
              <button onClick={() => runDeleteAction('trash')} className="dialog-trash danger">
                <strong>{t('dialog.deleteMoveTrash')}</strong>
                <span>{t('dialog.deleteMoveTrashDesc')}</span>
              </button>
              <button onClick={() => setDeleteDialog('none')} className="ghost">
                {t('dialog.cancel')}
              </button>
            </div>
          </div>
        </div>
      )}

      {pendingImport && (
        <div className="dialog-overlay">
          <div className="dialog import-dialog">
            <h3>{t('import.titulo')}</h3>
            <p className="edit-hint">{pendingImport.dir}</p>
            <label className="import-option">
              <input
                type="checkbox"
                checked={pendingImport.includeSubdirs}
                onChange={(e) =>
                  setPendingImport({ ...pendingImport, includeSubdirs: e.target.checked })
                }
              />
              {t('import.incluirSubpastas')}
            </label>
            <div className="import-option">
              <span>{t('import.tipoFotos')}</span>
              <div className="import-types">
                {(
                  [
                    ['all', t('import.todos')],
                    ['raw', t('app.raw')],
                    ['jpeg', t('app.jpeg')]
                  ] as Array<[string, string]>
                ).map(([v, label]) => (
                  <button
                    key={v}
                    className={`${pendingImport.types === v ? 'active' : ''}`}
                    onClick={() => setPendingImport({ ...pendingImport, types: v })}
                  >
                    {label}
                  </button>
                ))}
              </div>
            </div>
            <div className="import-option">
              <span>{t('import.tipoSessao')}</span>
              <select
                className="import-session"
                value={pendingImport.sessionType}
                onChange={(e) =>
                  setPendingImport({ ...pendingImport, sessionType: e.target.value })
                }
              >
                <option value="">{t('import.sessaoNenhum')}</option>
                {SESSION_TYPES.map((s) => (
                  <option key={s.id} value={s.id}>
                    {t(s.labelKey)}
                  </option>
                ))}
              </select>
            </div>
            <div className="import-actions">
              <button onClick={() => setPendingImport(null)} className="ghost">
                {t('dialog.cancel')}
              </button>
              <button onClick={() => confirmImport(pendingImport)} className="primary">
                {t('import.iniciar')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}