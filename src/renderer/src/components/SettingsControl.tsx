import { useEffect, useRef, useState } from 'react'
import { THEME_OPTIONS, useTheme, type AppearanceId, type ThemeId } from '../theme/ThemeContext'

const APPEARANCE_OPTIONS: Array<{ id: AppearanceId; label: string; description: string }> = [
  { id: 'dark', label: 'Escuro', description: 'Para editar fotos com menos brilho ao redor.' },
  { id: 'light', label: 'Claro', description: 'Para uma interface clara e luminosa.' }
]

export default function SettingsControl() {
  const [open, setOpen] = useState(false)
  const dialogRef = useRef<HTMLElement | null>(null)
  const triggerRef = useRef<HTMLButtonElement | null>(null)
  const { theme, setTheme, appearance, setAppearance } = useTheme()
  const [storage, setStorage] = useState<{ catalogDir: string; cacheDir: string; defaultCatalogDir: string; defaultCacheDir: string } | null>(null)
  const [storageBusy, setStorageBusy] = useState(false)
  const [storageMessage, setStorageMessage] = useState<string | null>(null)

  const close = () => {
    setOpen(false)
    requestAnimationFrame(() => triggerRef.current?.focus())
  }

  useEffect(() => {
    if (!open) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        close()
        return
      }
      if (event.key === 'Tab' && dialogRef.current) {
        const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>('button:not([disabled]), [href], input:not([disabled]), select:not([disabled])'))
        if (!focusable.length) return
        const first = focusable[0]
        const last = focusable[focusable.length - 1]
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault()
          last.focus()
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault()
          first.focus()
        }
      }
    }
    window.addEventListener('keydown', onKeyDown)
    requestAnimationFrame(() => dialogRef.current?.querySelector<HTMLElement>('button')?.focus())
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [open])

  useEffect(() => {
    if (!open) return
    setStorageMessage(null)
    window.openshoot.getStorageSettings().then((settings) => setStorage(settings)).catch((error) => setStorageMessage(String(error)))
  }, [open])

  const chooseStorageDirectory = async (kind: 'catalog' | 'cache') => {
    const selected = await window.openshoot.pickStorageDirectory(kind)
    if (!selected) return
    setStorage((current) => current ? { ...current, [kind === 'catalog' ? 'catalogDir' : 'cacheDir']: selected } : current)
  }

  const saveStorage = async () => {
    if (!storage) return
    setStorageBusy(true)
    setStorageMessage(null)
    try {
      const result = await window.openshoot.saveStorageSettings({ catalogDir: storage.catalogDir, cacheDir: storage.cacheDir })
      if (!result.ok) {
        setStorageMessage(result.error ?? 'Não foi possível salvar o armazenamento.')
      } else if (result.restartRequired) {
        setStorageMessage('Catálogo salvo. Reinicie o OpenShoot para usar a nova localização.')
      } else {
        setStorageMessage('Configuração de armazenamento salva.')
      }
    } catch (error) {
      setStorageMessage(String(error))
    } finally {
      setStorageBusy(false)
    }
  }

  const exportCatalog = async () => {
    setStorageBusy(true)
    setStorageMessage(null)
    try {
      const result = await window.openshoot.exportCatalogJson()
      setStorageMessage(result.ok ? `Catálogo exportado em ${result.path}` : (result.error ?? 'Exportação cancelada.'))
    } catch (error) {
      setStorageMessage(String(error))
    } finally {
      setStorageBusy(false)
    }
  }

  const importCatalog = async () => {
    setStorageBusy(true)
    setStorageMessage(null)
    try {
      const result = await window.openshoot.importCatalogJson()
      if (!result.ok) {
        setStorageMessage(result.error ?? 'Importação cancelada.')
      } else {
        setStorageMessage(`Importação concluída: ${result.albums_imported ?? 0} álbum(s), ${result.photos_linked ?? 0} vínculo(s), ${result.photos_updated ?? 0} foto(s) atualizada(s). ${result.photos_missing ?? 0} foto(s) não encontrada(s).`)
      }
    } catch (error) {
      setStorageMessage(String(error))
    } finally {
      setStorageBusy(false)
    }
  }

  return (
    <>
      <button
        type="button"
        ref={triggerRef}
        className="ghost settings-trigger"
        aria-label="Abrir configurações"
        title="Configurações"
        onClick={() => setOpen(true)}
      >
        <span aria-hidden="true">⚙</span>
        <span>Configurações</span>
      </button>

      {open && (
        <div className="dialog-overlay settings-overlay" onMouseDown={close}>
          <section
            ref={dialogRef}
            className="dialog settings-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="settings-title"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="settings-dialog-header">
              <div>
                <p className="settings-kicker">OpenShoot</p>
                <h2 id="settings-title">Configurações de aparência</h2>
              </div>
              <button type="button" className="ghost settings-close" onClick={close} aria-label="Fechar configurações">✕</button>
            </div>

            <section className="settings-section" aria-labelledby="appearance-title">
              <div>
                <h3 id="appearance-title">Modo da interface</h3>
                <p>Escolha a luminosidade adequada ao seu espaço de trabalho.</p>
              </div>
              <div className="appearance-options" role="group" aria-label="Modo da interface">
                {APPEARANCE_OPTIONS.map((option) => (
                  <button
                    type="button"
                    key={option.id}
                    className={`appearance-option ${appearance === option.id ? 'active' : ''}`}
                    aria-pressed={appearance === option.id}
                    onClick={() => setAppearance(option.id)}
                  >
                    <span className={`appearance-preview ${option.id}`} aria-hidden="true" />
                    <span>{option.label}</span>
                    <small>{option.description}</small>
                  </button>
                ))}
              </div>
            </section>

            <section className="settings-section" aria-labelledby="theme-title">
              <div>
                <h3 id="theme-title">Cor do tema</h3>
                <p>Café é o padrão do OpenShoot. Sua preferência é salva neste computador.</p>
              </div>
              <div className="theme-options" role="radiogroup" aria-label="Cor do tema">
                {THEME_OPTIONS.map((option) => (
                  <button
                    type="button"
                    key={option.id}
                    role="radio"
                    aria-checked={theme === option.id}
                    className={`theme-option ${option.id} ${theme === option.id ? 'active' : ''}`}
                    onClick={() => setTheme(option.id as ThemeId)}
                  >
                    <span className="theme-option-swatch" aria-hidden="true" />
                    {option.label}
                  </button>
                ))}
              </div>
            </section>

            <section className="settings-section settings-storage" aria-labelledby="storage-title">
              <div>
                <h3 id="storage-title">Armazenamento</h3>
                <p>O catálogo guarda organização, pessoas, avaliações e edições. As fotos originais não são movidas.</p>
              </div>
              {storage && (
                <div className="settings-storage-fields">
                  <div className="settings-path-field">
                    <span>Catálogo SQLite</span>
                    <code title={storage.catalogDir}>{storage.catalogDir}</code>
                    <button type="button" className="ghost small" onClick={() => chooseStorageDirectory('catalog')} disabled={storageBusy}>Alterar</button>
                  </div>
                  <div className="settings-path-field">
                    <span>Cache de previews</span>
                    <code title={storage.cacheDir}>{storage.cacheDir}</code>
                    <button type="button" className="ghost small" onClick={() => chooseStorageDirectory('cache')} disabled={storageBusy}>Alterar</button>
                  </div>
                  <div className="settings-storage-actions">
                    <button type="button" onClick={saveStorage} disabled={storageBusy}>Salvar armazenamento</button>
                    <button type="button" className="ghost" onClick={exportCatalog} disabled={storageBusy}>Exportar catálogo JSON</button>
                    <button type="button" className="ghost" onClick={importCatalog} disabled={storageBusy}>Importar catálogo JSON</button>
                  </div>
                  {storageMessage && <p className="settings-storage-message" role="status">{storageMessage}</p>}
                </div>
              )}
            </section>

            <div className="settings-dialog-actions">
              <button type="button" onClick={close}>Concluído</button>
            </div>
          </section>
        </div>
      )}
    </>
  )
}
