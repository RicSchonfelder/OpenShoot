import { useEffect, useState } from 'react'
import { useT } from '../i18n/I18nContext'

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
  edited: number
}

export default function FilterPanel({ active, onSelect, photoIds }: FilterPanelProps) {
  const { t } = useT()
  const [counts, setCounts] = useState<Counts | null>(null)

  useEffect(() => {
    let activeFlag = true
    const load = () => window.openshoot.filterCounts(photoIds).then((c) => {
      if (activeFlag && c) setCounts(c)
    })
    load()
    // Atualiza ao reabrir/mudar (cada loadPhotos muda o estado do App → re-render).
    const id = setInterval(() => load(), 3000)
    return () => {
      activeFlag = false
      clearInterval(id)
    }
  }, [photoIds])

  const items: Array<{ key: FilterKey; label: string; icon: string; count: number }> = [
    { key: 'all', label: t('app.todos'), icon: '☰', count: counts?.all ?? 0 },
    { key: 'picks', label: t('app.picks'), icon: 'P', count: counts?.picks ?? 0 },
    { key: 'rejects', label: t('app.rejects'), icon: 'X', count: counts?.rejects ?? 0 },
    { key: 'unrated', label: t('app.unrated'), icon: 'U', count: counts?.unrated ?? 0 },
    { key: 'review', label: t('app.paraRevisao'), icon: '?', count: counts?.review ?? 0 },
    { key: 'destaques', label: t('app.destaques'), icon: '★', count: counts?.destaques ?? 0 },
    { key: 'selecionado', label: t('app.selecionado'), icon: '✔', count: counts?.selecionado ?? 0 },
    { key: 'duplicates', label: t('app.duplicatas'), icon: '⧉', count: counts?.duplicates ?? 0 },
    { key: 'faces', label: t('app.comRosto'), icon: '◉', count: counts?.faces ?? 0 },
    { key: 'edited', label: t('app.editadas'), icon: '✎', count: counts?.edited ?? 0 }
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
