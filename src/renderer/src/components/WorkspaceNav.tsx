import { useT } from '../i18n/I18nContext'

export type WorkspaceSection = 'import' | 'cull' | 'edit' | 'retouch' | 'export'

interface WorkspaceNavProps {
  active: WorkspaceSection
  onNavigate: (section: WorkspaceSection) => void
}

/** Navegação primária fixa para todos os fluxos de trabalho com fotos. */
export default function WorkspaceNav({ active, onNavigate }: WorkspaceNavProps) {
  const { t } = useT()
  const sections: Array<{ id: WorkspaceSection; label: string }> = [
    { id: 'import', label: t('app.modeImport') },
    { id: 'cull', label: t('app.modeCull') },
    { id: 'edit', label: t('app.modeEdit') },
    { id: 'retouch', label: t('app.modeRetouch') },
    { id: 'export', label: t('export.exportarBtn') }
  ]

  return (
    <nav className="mode-tabs workspace-nav" aria-label="Área de trabalho">
      {sections.map(({ id, label }) => (
        <button
          key={id}
          type="button"
          className={`mode-tab ${active === id ? 'active' : ''}`}
          aria-current={active === id ? 'page' : undefined}
          onClick={() => onNavigate(id)}
        >
          {label}
        </button>
      ))}
    </nav>
  )
}
