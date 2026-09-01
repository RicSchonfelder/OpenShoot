import { useEffect, useRef, useState } from 'react'
import { THEME_OPTIONS, useTheme, type AppearanceId, type ThemeId } from '../theme/ThemeContext'

const APPEARANCE_OPTIONS: Array<{ id: AppearanceId; label: string; description: string }> = [
  { id: 'dark', label: 'Escuro', description: 'Para editar fotos com menos brilho ao redor.' },
  { id: 'light', label: 'Claro', description: 'Para uma interface clara e luminosa.' }
]

interface PresetItem {
  name: string
  recipe: string
  file_type?: string
  color_type?: string
  source?: string
}

const PRESET_SOURCES: Record<string, string> = {
  manual: 'Manual',
  lightroom: 'Lightroom',
  imported: 'Importado',
  learned: 'Aprendido'
}

const PRESET_CATEGORIES: Array<{ key: string; label: string; icon: string }> = [
  { key: 'b&w', label: 'Preto & Branco', icon: '◐' },
  { key: 'color', label: 'Cor', icon: '🎨' },
  { key: 'creative', label: 'Criativo', icon: '✦' },
  { key: 'curve', label: 'Curva', icon: '∿' },
  { key: 'grain', label: 'Granulação', icon: '≋' },
  { key: 'vignetting', label: 'Vinheta', icon: '◍' },
  { key: 'portraits', label: 'Retratos', icon: '👤' },
  { key: 'sharpening', label: 'Nitidez', icon: '△' },
  { key: 'other', label: 'Outros', icon: '•' }
]

function presetCategory(name: string): string {
  const lower = name.toLowerCase()
  for (const c of PRESET_CATEGORIES) {
    if (c.key === 'other') continue
    if (lower.startsWith(c.key)) return c.key
    if (lower.includes(c.key)) return c.key
  }
  return 'other'
}

export default function SettingsControl() {
  const [open, setOpen] = useState(false)
  const dialogRef = useRef<HTMLElement | null>(null)
  const triggerRef = useRef<HTMLButtonElement | null>(null)
  const { theme, setTheme, appearance, setAppearance } = useTheme()
  const [storage, setStorage] = useState<{ catalogDir: string; cacheDir: string; defaultCatalogDir: string; defaultCacheDir: string } | null>(null)
  const [storageBusy, setStorageBusy] = useState(false)
  const [storageMessage, setStorageMessage] = useState<string | null>(null)
  const [presets, setPresets] = useState<PresetItem[]>([])
  const [presetBusy, setPresetBusy] = useState(false)
  const [presetMessage, setPresetMessage] = useState<string | null>(null)

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

  useEffect(() => {
    if (!open) return
    setPresetMessage(null)
    window.openshoot.listPresets().then((p) => setPresets(p)).catch(() => setPresets([]))
  }, [open])

  const loadPresets = () => {
    window.openshoot.listPresets().then((p) => setPresets(p)).catch(() => setPresets([]))
  }

  const importPresetFile = async () => {
    const path = await window.openshoot.pickPresetFile()
    if (!path) return
    setPresetBusy(true)
    setPresetMessage(null)
    try {
      const res = await window.openshoot.importLightroomPreset(path)
      if (res.ok && res.name) {
        setPresetMessage(`Preset '${res.name}' importado.`)
        loadPresets()
      } else {
        setPresetMessage(res.error ?? 'Não foi possível importar o preset.')
      }
    } catch (error) {
      setPresetMessage(String(error))
    } finally {
      setPresetBusy(false)
    }
  }

  const importPresetFolder = async () => {
    const dir = await window.openshoot.pickFolder()
    if (!dir) return
    setPresetBusy(true)
    setPresetMessage(null)
    try {
      const res = await window.openshoot.importLightroomFolder(dir)
      if (res.ok) {
        const n = res.imported?.length ?? 0
        const errs = res.errors?.length ?? 0
        setPresetMessage(`${n} preset(s) importado(s)${errs > 0 ? `, ${errs} erro(s)` : ''}.`)
        loadPresets()
      } else {
        setPresetMessage(res.error ?? 'Não foi possível importar a pasta.')
      }
    } catch (error) {
      setPresetMessage(String(error))
    } finally {
      setPresetBusy(false)
    }
  }

  const importPresetJson = async () => {
    const path = await window.openshoot.pickPresetJson()
    if (!path) return
    setPresetBusy(true)
    setPresetMessage(null)
    try {
      const res = await window.openshoot.importPresetFromFile(path)
      if (res.ok) {
        setPresetMessage(`Preset '${res.name}' importado.`)
        loadPresets()
      } else {
        setPresetMessage(res.error ?? 'Não foi possível importar o preset.')
      }
    } catch (error) {
      setPresetMessage(String(error))
    } finally {
      setPresetBusy(false)
    }
  }

  const removePreset = async (name: string) => {
    await window.openshoot.deletePreset(name)
    loadPresets()
  }

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

            <section className="settings-section settings-presets" aria-labelledby="presets-title">
              <div>
                <h3 id="presets-title">Presets de edição</h3>
                <p>Instale presets do Lightroom (.xmp/.lrtemplate), de pastas inteiras ou arquivos JSON. Eles ficam disponíveis no painel de edição.</p>
              </div>
              <div className="settings-preset-actions">
                <button type="button" onClick={importPresetFile} disabled={presetBusy}>
                  <span aria-hidden="true">＋</span> Importar preset Lightroom
                </button>
                <button type="button" className="ghost" onClick={importPresetFolder} disabled={presetBusy}>
                  <span aria-hidden="true">📁</span> Importar pasta
                </button>
                <button type="button" className="ghost" onClick={importPresetJson} disabled={presetBusy}>
                  <span aria-hidden="true">⇄</span> Importar JSON
                </button>
              </div>
              {presetMessage && <p className="settings-storage-message" role="status">{presetMessage}</p>}
              {presets.length > 0 ? (
                <div className="settings-preset-browser">
                  <div className="settings-preset-summary">
                    <span className="settings-preset-total"><strong>{presets.length}</strong> presets</span>
                    {PRESET_CATEGORIES.map((cat) => {
                      const count = presets.filter((p) => presetCategory(p.name) === cat.key).length
                      return count > 0 ? (
                        <span key={cat.key} className="settings-preset-cat-count" data-cat={cat.key}>
                          {cat.icon} {count}
                        </span>
                      ) : null
                    })}
                  </div>
                  <div className="settings-preset-groups">
                    {PRESET_CATEGORIES.map((cat) => {
                      const items = presets.filter((p) => presetCategory(p.name) === cat.key)
                      if (items.length === 0) return null
                      return (
                        <details key={cat.key} className="settings-preset-group" open={items.length <= 10}>
                          <summary>
                            <span className="settings-preset-group-icon">{cat.icon}</span>
                            <span className="settings-preset-group-label">{cat.label}</span>
                            <span className="settings-preset-group-count">{items.length}</span>
                          </summary>
                          <ul className="settings-preset-list">
                            {items.map((p) => (
                              <li key={p.name}>
                                <span className="settings-preset-name" title={p.name}>{p.name}</span>
                                <span className="settings-preset-meta">
                                  {p.source && PRESET_SOURCES[p.source] && (
                                    <em title={`Origem: ${p.source}`}>{PRESET_SOURCES[p.source]}</em>
                                  )}
                                  {p.file_type === 'raw' || p.file_type === 'jpeg' ? <em>{p.file_type.toUpperCase()}</em> : null}
                                  {p.color_type === 'color' ? <em>Cor</em> : null}
                                  {p.color_type === 'bw' ? <em>P&B</em> : null}
                                </span>
                                <button type="button" className="ghost small settings-preset-del" onClick={() => removePreset(p.name)} aria-label={`Remover ${p.name}`}>✕</button>
                              </li>
                            ))}
                          </ul>
                        </details>
                      )
                    })}
                  </div>
                </div>
              ) : (
                <p className="settings-storage-message">Nenhum preset instalado ainda. Use os botões acima para importar.</p>
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
