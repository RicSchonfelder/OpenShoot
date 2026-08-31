import { useCallback, useEffect, useRef, useState } from 'react'
import Gallery from './components/Gallery'
import EditPanel from './components/EditPanel'
import EditViewPhoto from './components/EditViewPhoto'
import EditViewFilmstrip from './components/EditViewFilmstrip'
import ExportView from './components/ExportView'
import LoupeView from './components/LoupeView'
import FilterPanel from './components/FilterPanel'
import HomeView from './components/HomeView'
import PeopleView from './components/PeopleView'
import RestorerView from './components/RestorerView'
import SettingsControl from './components/SettingsControl'
import WorkspaceNav, { type WorkspaceSection } from './components/WorkspaceNav'
import { useT } from './i18n/I18nContext'
import type { PhotoMeta, PersistedFace } from '../../types/photo'

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
  const [editViewId, setEditViewId] = useState<number | null>(null)
  const [editPreview, setEditPreview] = useState<{ id: number; src: string } | null>(null)
  const [editCompareMode, setEditCompareMode] = useState<'original' | 'modified' | 'side-by-side' | 'slider'>('modified')
  const [currentAlbum, setCurrentAlbum] = useState<number | null>(null)
  const [albumPhotoIds, setAlbumPhotoIds] = useState<Set<number> | null>(null)
  const [mode, setMode] = useState<'import' | 'cull' | 'edit' | 'retouch'>('import')
  const [exportScopeIds, setExportScopeIds] = useState<number[] | null>(null)
  const [showPeople, setShowPeople] = useState(false)
  const [showRestorer, setShowRestorer] = useState(false)
  const [showExportView, setShowExportView] = useState(false)
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
  const [photoFaces, setPhotoFaces] = useState<PersistedFace[]>([])
  const [oneClickPresetName, setOneClickPresetName] = useState('')
  const [appPresets, setAppPresets] = useState<Array<{ name: string; recipe: string }>>([])

  // A navegação primária é única na área de trabalho. Trocar de seção fecha
  // somente visualizações transitórias e preserva o álbum em uso.
  const navigateWorkspace = useCallback((section: WorkspaceSection) => {
    setMoreOpen(false)
    setShowPeople(false)
    setShowRestorer(false)
    setLoupeOpen(false)
    setEditViewId(null)
    setExportScopeIds(null)
    setShowExportView(section === 'export')
    if (section !== 'export') {
      setMode(section)
    }
    if (section === 'cull' || section === 'edit' || section === 'retouch' || section === 'import') {
      window.openshoot.listPresets().then((p) => setAppPresets(p)).catch(() => {})
    }
  }, [])
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
        // Se estiver num álbum, filtra pelas fotos do álbum.
        if (currentAlbum != null && albumPhotoIds) {
          const inAlbum = list.photos.filter((p) => albumPhotoIds.has(p.id))
          setPhotos(inAlbum)
        } else {
          setPhotos(list.photos)
        }
        setSelectedIds(new Set())
        setAnchorId(null)
      } catch (e) {
        setError(String(e))
      }
    },
    [filter, currentAlbum, albumPhotoIds]
  )

  useEffect(() => {
    loadPhotos()
  }, [loadPhotos, filter])

  useEffect(() => {
    window.openshoot.listPresets().then((p) => setAppPresets(p)).catch(() => {})
  }, [])

  // Abre um álbum: carrega os ids e volta para a galeria filtrada.
  const openAlbum = useCallback(async (albumId: number) => {
    const ids = await window.openshoot.albumPhotoIds(albumId)
    setAlbumPhotoIds(new Set(ids))
    setCurrentAlbum(albumId)
    setEditViewId(null)
  }, [])

  const closeAlbum = useCallback(() => {
    setCurrentAlbum(null)
    setAlbumPhotoIds(null)
    setSelectedIds(new Set())
    setAnchorId(null)
    loadPhotos()
  }, [loadPhotos])

  const selectedPhoto = photos.find((p) => p.id === anchorId) ?? null
  const handleEditPreviewChange = useCallback((src: string | null) => {
    setEditPreview(src && editViewId != null ? { id: editViewId, src } : null)
  }, [editViewId])

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
        setEditViewId(null)
      } else if (opts.toggle) {
        // Cmd/Ctrl: alterna este item mantendo os outros
        const next = new Set(selectedRef.current)
        if (next.has(id)) next.delete(id)
        else next.add(id)
        setSelectedIds(next)
        setEditViewId(null)
      } else {
        // Clique simples apenas seleciona. Abrir é uma ação explícita
        // (duplo clique ou Enter), especialmente importante no culling.
        setSelectedIds(new Set([id]))
        setEditViewId(null)
      }
      setAnchorId(id)
    },
    []
  )

  const handleActivate = useCallback((id: number) => {
    const idx = photosRef.current.findIndex((p) => p.id === id)
    if (mode === 'import') {
      if (idx >= 0) {
        setLoupeIndex(idx)
        setLoupeOpen(true)
      }
      setAnchorId(id)
      setSelectedIds(new Set([id]))
      return
    }
    if (mode === 'edit' || mode === 'retouch') {
      setEditViewId(id)
      setSelectedIds(new Set([id]))
      setAnchorId(id)
      return
    }
    if (idx >= 0) {
      setLoupeIndex(idx)
      setLoupeOpen(true)
    }
    setAnchorId(id)
    setSelectedIds(new Set([id]))
    }, [mode])

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
      const scoped = currentAlbum != null && albumPhotoIds
        ? updated.photos.filter((p) => albumPhotoIds.has(p.id))
        : updated.photos
      setPhotos(scoped)
      if (advance && loupeIndex < scoped.length - 1) {
        setLoupeIndex(loupeIndex + 1)
      } else {
        setAnchorId(photo.id)
      }
    },
    [loupeIndex, filter, currentAlbum, albumPhotoIds, persistRating]
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
      if (filter === 'all') {
        setPhotos((current) => current.map((photo) => photo.id === id ? { ...photo, rating } : photo))
      } else {
        await loadPhotos()
      }
    },
    [filter, persistRating, loadPhotos]
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
      // Enter / Espaço abre a ferramenta apropriada ao modo atual.
      if (e.key === 'Enter' || e.key === ' ') {
        if (anchor != null && sel.size > 0) {
          e.preventDefault()
          const idx = photos.findIndex((p) => p.id === anchor)
          if (idx >= 0) {
            if (mode === 'cull' || mode === 'import') {
              setLoupeIndex(idx)
              setLoupeOpen(true)
            } else {
              setEditViewId(anchor)
            }
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
        // Se está num álbum, associa as fotos importadas ao álbum.
        if (currentAlbum != null) {
          await window.openshoot.addFolderToAlbum(currentAlbum, opts.dir)
          const ids = await window.openshoot.albumPhotoIds(currentAlbum)
          setAlbumPhotoIds(new Set(ids))
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
    [loadPhotos, t, currentAlbum]
  )

  const runCull = useCallback(async () => {
    setError(null)
    setCulling(true)
    try {
      const res = await window.openshoot.cullPhotos(targetPicks > 0 ? targetPicks : undefined, albumPhotoIds ? Array.from(albumPhotoIds) : undefined)
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
  }, [albumPhotoIds, loadPhotos, t, targetPicks])

  const runOneClick = useCallback(async () => {
    setError(null)
    setScanMsg(null)
    setCulling(true)
    try {
      // 1) Cull com meta (se definida) — seleção automática.
      const res = await window.openshoot.cullPhotos(targetPicks > 0 ? targetPicks : undefined, albumPhotoIds ? Array.from(albumPhotoIds) : undefined)
      if ('error' in res) {
        setError(String(res.error))
        return
      }
      // 2) Aplica o preset selecionado (se houver).
      const presets = await window.openshoot.listPresets()
      const selectedPreset = presets.find((p) => p.name === oneClickPresetName) ?? null
      if (selectedPreset) {
        const picked = await window.openshoot.listPhotos('', 'picks', 0, PAGE_SIZE)
        const scopedPicks = albumPhotoIds ? picked.photos.filter((photo) => albumPhotoIds.has(photo.id)) : picked.photos
        let n = 0
        for (const photo of scopedPicks) {
          await window.openshoot.setPhotoEdit(photo.id, selectedPreset.recipe)
          await window.openshoot.writeXmpForPhoto(photo.id)
          n++
        }
        setScanMsg(
          t('app.oneClickMsg', { picks: res.picks, editadas: n, preset: selectedPreset.name })
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
  }, [albumPhotoIds, loadPhotos, t, targetPicks, oneClickPresetName])

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
      const ids = selectedIds.size > 0 ? Array.from(selectedIds) : photos.map((p) => p.id)
      if (ids.length === 0) return
      let exported = 0
      let errors = 0
      for (const id of ids) {
        try {
          await window.openshoot.writeXmpForPhoto(id)
          exported++
        } catch {
          errors++
        }
      }
      setScanMsg(
        errors > 0
          ? t('app.xmpMsgErr', { exported, errors })
          : t('app.xmpMsg', { exported })
      )
    } catch (e) {
      setError(String(e))
    } finally {
      setExporting(false)
    }
  }, [selectedIds, photos, t])

  const handleApplyAll = useCallback(
    async (json: string, ids: number[]) => {
      if (ids.length === 0) return
      try {
        let applied = 0
        for (const id of ids) {
          await window.openshoot.setPhotoEdit(id, json)
          await window.openshoot.writeXmpForPhoto(id)
          applied++
        }
        setScanMsg(t('app.editMsg', { applied }))
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

  const openInEdit = useCallback((photoId: number) => {
    setEditViewId(photoId)
    setSelectedIds(new Set([photoId]))
    setAnchorId(photoId)
    setMode('edit')
    setShowPeople(false)
  }, [])

  const openInRetouch = useCallback((photoId: number) => {
    setEditViewId(photoId)
    setSelectedIds(new Set([photoId]))
    setAnchorId(photoId)
    setMode('retouch')
    setShowPeople(false)
  }, [])

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

  // Carrega faces da foto aberta no modo edição.
  useEffect(() => {
    if (editViewId == null) {
      setPhotoFaces([])
      return
    }
    let active = true
    window.openshoot
      .listFacesForPhoto(editViewId)
      .then((res) => {
        if (active) setPhotoFaces(res.ok && res.faces ? res.faces : [])
      })
      .catch(() => {
        if (active) setPhotoFaces([])
      })
    return () => { active = false }
  }, [editViewId])

  // Atalhos do modo de edição em tela grande.
  useEffect(() => {
    if (editViewId == null) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setEditViewId(null)
        return
      }
      if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') {
        const idx = photosRef.current.findIndex((p) => p.id === editViewId)
        if (idx < 0) return
        const dir = e.key === 'ArrowRight' ? 1 : -1
        const next = photosRef.current[idx + dir]
        if (next) {
          setEditViewId(next.id)
          setSelectedIds(new Set([next.id]))
          setAnchorId(next.id)
        }
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [editViewId])

  // A exportação é uma área de trabalho própria. Ela não pode ser renderizada
  // sobre a galeria, pois isso deixava dois cabeçalhos visíveis e permitia
  // interação acidental com a tela de trás.
  if (showExportView) {
    const exportPhotos = exportScopeIds ? photos.filter((photo) => exportScopeIds.includes(photo.id)) : photos
    return (
      <ExportView
        photos={exportPhotos}
        scope={exportScopeIds ? 'selection' : 'visible'}
        onClose={() => {
          setShowExportView(false)
          setExportScopeIds(null)
        }}
        onNavigate={navigateWorkspace}
      />
    )
  }

  if (showRestorer) return <RestorerView onBack={() => setShowRestorer(false)} onNavigate={navigateWorkspace} photoIds={currentAlbum != null && albumPhotoIds ? Array.from(albumPhotoIds) : undefined} />

  // Tela Lar (álbuns) quando não há álbum aberto.
  if (currentAlbum == null) {
    return <HomeView onOpenAlbum={openAlbum} />
  }

  // Tela Pessoas (agrupamento facial) em tela cheia.
  if (showPeople && currentAlbum != null) {
    return (
      <PeopleView
        albumId={currentAlbum}
        activeSection="cull"
        onBack={() => setShowPeople(false)}
        onNavigate={navigateWorkspace}
        onOpenEdit={openInEdit}
        onOpenRetouch={openInRetouch}
      />
    )
  }

  if (loupeOpen) {
    return (
      <LoupeView
        photos={photos}
        currentIndex={loupeIndex}
        onNavigate={loupeNavigate}
        onApplyRating={loupeApplyRating}
        onClose={loupeClose}
        onNavigateWorkspace={navigateWorkspace}
      />
    )
  }

  // Modo de edição em tela grande: foto por inteiro + painel de edição.
  const editPhoto = editViewId != null ? (photos.find((p) => p.id === editViewId) ?? null) : null
  if (editPhoto) {
    const selectEditPhoto = (id: number) => {
      setEditViewId(id)
      setSelectedIds(new Set([id]))
      setAnchorId(id)
    }
    return (
      <div className="app">
        <header className="topbar workspace-topbar">
          <div className="workspace-left">
            <span className="logo">OpenShoot</span>
            <button onClick={() => setEditViewId(null)} className="ghost">
              {t('app.voltarGaleria')}
            </button>
          </div>
          <WorkspaceNav active={mode} onNavigate={navigateWorkspace} />
          <div className="topbar-right workspace-actions">
            <SettingsControl />
            <button onClick={exportXmp} disabled={exporting} className="ghost" title={t('export.exportMetadataHint')}>
              {exporting ? t('app.exportando') : t('export.exportMetadata')}
            </button>
          </div>
        </header>
        <main className="content editview">
          <div className="editview-stage">
            <div className="editview-photo-area">
            <EditViewPhoto photoId={editPhoto.id} modifiedSrc={editPreview?.id === editPhoto.id ? editPreview.src : null} compareMode={editCompareMode} onCompareModeChange={setEditCompareMode} photoFaces={photoFaces} />
            </div>
            <EditViewFilmstrip photos={photos} activeId={editPhoto.id} onSelect={selectEditPhoto} />
          </div>
            <EditPanel photo={editPhoto} variant={mode === 'retouch' ? 'retouch' : 'edit'} selectedIds={selectedIds} onApplyAll={handleApplyAll} onPreviewChange={handleEditPreviewChange} onModification={() => setEditCompareMode('modified')} photoFaces={photoFaces} />
        </main>
        <div className="shortcuts-bar">
          <span dangerouslySetInnerHTML={{ __html: t('app.editViewShortcuts') }} />
        </div>
      </div>
    )
  }

  return (
    <div className="app">
      <header className="topbar workspace-topbar">
        <div className="workspace-left">
          <span className="logo">OpenShoot</span>
          <button onClick={closeAlbum} className="ghost back-albums">
            ← {t('app.meusAlbums')}
          </button>
        </div>
        <WorkspaceNav active={mode} onNavigate={navigateWorkspace} />
        <div className="topbar-right workspace-actions">
          <SettingsControl />
        </div>
      </header>

      <div className="workspace-contextbar" aria-label="Ferramentas da seção">
          {mode !== 'import' && mode !== 'cull' && (
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
        )}
          {mode === 'edit' && (
            <button onClick={() => setShowRestorer(true)} className="ghost">
              Bancada de restauração
            </button>
          )}
          {selectedIds.size > 0 && (
            <button onClick={deleteSelected} className="danger">
              {t('app.deletar')} ({selectedIds.size})
            </button>
          )}
          {(mode === 'edit' || mode === 'retouch') && (
            <>
              <button
                onClick={() => {
                  setExportScopeIds(selectedIds.size > 0 ? Array.from(selectedIds) : null)
                  setShowExportView(true)
                }}
                className="ghost"
                title={t('export.hint')}
              >
                {selectedIds.size > 0
                  ? t('export.exportar', { n: selectedIds.size })
                  : t('export.exportar', { n: photos.length })}
              </button>
              <button onClick={exportXmp} disabled={exporting || photos.length === 0} className="ghost" title={t('export.exportMetadataHint')}>
                {exporting ? t('app.exportando') : t('export.exportMetadata')}
              </button>
            </>
          )}
          {mode === 'cull' && (
            <>
              <button onClick={runCull} disabled={culling || photos.length === 0} className="primary">
                {culling ? t('app.cullRun') : t('app.cull')}
              </button>
              <button onClick={() => setShowPeople(true)} disabled={photos.length === 0} className="ghost">
                {t('people.titulo')}
              </button>
            </>
          )}
          {mode === 'import' && (
            <>
              <button onClick={importFolder} disabled={scanning}>
                {scanning ? t('app.importando') : t('app.importar')}
              </button>
            </>
          )}
      </div>

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

      {mode === 'cull' && (
      <div className="cull-toolbar">
        <div className="cull-toolbar-summary" aria-label="Resumo da seleção">
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
        </div>
        <div className="cull-toolbar-actions">
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
        <select
          className="toolbar-preset-select"
          value={oneClickPresetName}
          onChange={(e) => setOneClickPresetName(e.target.value)}
          title={t('app.oneClickHintPreset')}
        >
          <option value="">{t('app.oneClickSemPreset')}</option>
          {appPresets.map((p) => (
            <option key={p.name} value={p.name}>{p.name}</option>
          ))}
        </select>
        <button
          className="toolbar-one-click"
          onClick={runOneClick}
          disabled={culling || photos.length === 0}
          title={oneClickPresetName ? t('app.oneClickHintPreset') : t('app.oneClickHint')}
        >
          {culling ? t('app.importando') : t('app.oneClick')}
        </button>
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
      </div>
      )}

      <main className="content">
        <div className="main-gallery">
          {mode === 'cull' && (
            <FilterPanel
              active={filter}
              onSelect={(f) => setFilter(f as Filter)}
              photoIds={albumPhotoIds ? Array.from(albumPhotoIds) : undefined}
            />
          )}
          <Gallery
            photos={photos}
            onRefresh={loadPhotos}
            selectedIds={selectedIds}
            anchorId={anchorId}
            onSelect={handleSelect}
            onActivate={handleActivate}
            onRate={handleRate}
            mode={mode}
          />
        </div>
        {(mode === 'edit' || mode === 'retouch') && (
          <EditPanel photo={selectedPhoto} variant={mode === 'retouch' ? 'retouch' : 'edit'} onApplyAll={handleApplyAll} selectedIds={selectedIds} photoFaces={photoFaces} />
        )}
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
