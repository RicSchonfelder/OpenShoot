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

            <div className="settings-dialog-actions">
              <button type="button" onClick={close}>Concluído</button>
            </div>
          </section>
        </div>
      )}
    </>
  )
}
