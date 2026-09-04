import { useEffect, useMemo, useState } from 'react'
import { useT } from '../i18n/I18nContext'
import type { PhotoMeta } from '../../../types/photo'

export type FilterKey =
  | 'all'
  | 'picks'
  | 'rejects'
  | 'unrated'
  | 'review'
  | 'destaques'
  | 'selecionado'
  | 'duplicates'
  | 'faces'
  | 'eyes_warning'
  | 'edited'

interface FilterPanelProps {
  active: string
  onSelect: (f: FilterKey) => void
  photoIds?: number[]
}

interface Counts {
  all: number
  picks: number
  rejects: number
  unrated: number
  review: number
  destaques: number
  selecionado: number
  duplicates: number
  faces: number
  eyes_warning: number
  edited: number
}

export default function FilterPanel({ active, onSelect, photoIds }: FilterPanelProps) {
  const { t } = useT()
  const [counts, setCounts] = useState<Counts | null>(null)
  // `Array.from(albumIds)` é criado a cada render no App. Usar uma chave
  // estável evita reiniciar a consulta e o intervalo continuamente.
  const scopeKey = useMemo(() => photoIds?.join(',') ?? 'catalog', [photoIds])

  useEffect(() => {
    let activeFlag = true
    const scopedIds = scopeKey === 'catalog' ? null : new Set(scopeKey.split(',').map(Number))
    const load = async () => {
      const list = await window.openshoot.listPhotos('', 'all', 0, 1000)
      if (!activeFlag) return
      const photos = scopedIds ? list.photos.filter((photo) => scopedIds.has(photo.id)) : list.photos
      setCounts(buildCounts(photos))
    }
    load()
    // Atualiza ao reabrir/mudar (cada loadPhotos muda o estado do App → re-render).
    const id = setInterval(() => load(), 3000)
    return () => {
      activeFlag = false
      clearInterval(id)
    }
  }, [scopeKey])

  const items: Array<{ key: FilterKey; label: string; icon: string; count: number | string }> = [
    { key: 'all', label: t('app.todos'), icon: '☰', count: counts?.all ?? '—' },
    { key: 'picks', label: t('app.picks'), icon: 'P', count: counts?.picks ?? '—' },
    { key: 'rejects', label: t('app.rejects'), icon: 'X', count: counts?.rejects ?? '—' },
    { key: 'unrated', label: t('app.unrated'), icon: 'U', count: counts?.unrated ?? '—' },
    { key: 'review', label: t('app.paraRevisao'), icon: '?', count: counts?.review ?? '—' },
    { key: 'destaques', label: t('app.destaques'), icon: '★', count: counts?.destaques ?? '—' },
    { key: 'selecionado', label: t('app.selecionado'), icon: '✔', count: counts?.selecionado ?? '—' },
    { key: 'duplicates', label: t('app.duplicatas'), icon: '⧉', count: counts?.duplicates ?? '—' },
    { key: 'faces', label: t('app.comRosto'), icon: '◉', count: counts?.faces ?? '—' },
    { key: 'eyes_warning', label: t('app.olhosFechados'), icon: '◌', count: counts?.eyes_warning ?? '—' },
    { key: 'edited', label: t('app.editadas'), icon: '✎', count: counts?.edited ?? '—' }
  ]

  return (
    <aside className="filter-panel">
      <div className="filter-panel-header">
        <span>{t('app.filtrosTitulo')}</span>
        <button className="filter-reset" onClick={() => onSelect('all')}>
          {t('app.reiniciar')}
        </button>
      </div>
      <div className="filter-list">
        {items.map((it) => (
          <button
            key={it.key}
            className={`filter-item ${active === it.key ? 'active' : ''}`}
            onClick={() => onSelect(it.key)}
          >
            <span className={`filter-icon ${it.key}`}>{it.icon}</span>
            <span className="filter-label">{it.label}</span>
            <span className="filter-count">{it.count}</span>
          </button>
        ))}
      </div>
    </aside>
  )
}

function buildCounts(photos: PhotoMeta[]): Counts {
  const duplicateHashes = new Set<string>()
  const hashFrequency = new Map<string, number>()
  for (const photo of photos) {
    if (photo.hash) hashFrequency.set(photo.hash, (hashFrequency.get(photo.hash) ?? 0) + 1)
  }
  for (const [hash, count] of hashFrequency) if (count > 1) duplicateHashes.add(hash)

  return {
    all: photos.length,
    picks: photos.filter((photo) => photo.rating >= 4).length,
    rejects: photos.filter((photo) => photo.rating >= 1 && photo.rating <= 2).length,
    unrated: photos.filter((photo) => photo.rating === 0).length,
    review: photos.filter((photo) => photo.review).length,
    destaques: photos.filter((photo) => photo.aiPick).length,
    selecionado: photos.filter((photo) => photo.rating >= 4 && !photo.aiPick).length,
    duplicates: photos.filter((photo) => duplicateHashes.has(photo.hash)).length,
    faces: photos.filter((photo) => photo.hasFace).length,
    eyes_warning: photos.filter((photo) => photo.eyesScore != null && photo.eyesScore >= 0 && photo.eyesScore < 0.40).length,
    edited: photos.filter((photo) => photo.hasXmp).length
  }
}
