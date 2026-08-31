import { createContext, useContext, useEffect, useMemo, useState } from 'react'

export const THEME_OPTIONS = [
  { id: 'coffee', label: 'Café' },
  { id: 'graphite', label: 'Grafite' },
  { id: 'ocean', label: 'Oceano' },
  { id: 'forest', label: 'Floresta' }
] as const

export type ThemeId = (typeof THEME_OPTIONS)[number]['id']
export type AppearanceId = 'dark' | 'light'

const STORAGE_KEY = 'openshoot-theme'
const APPEARANCE_STORAGE_KEY = 'openshoot-appearance'
const DEFAULT_THEME: ThemeId = 'coffee'
const DEFAULT_APPEARANCE: AppearanceId = 'dark'

interface ThemeContextValue {
  theme: ThemeId
  setTheme: (theme: ThemeId) => void
  appearance: AppearanceId
  setAppearance: (appearance: AppearanceId) => void
}

const ThemeContext = createContext<ThemeContextValue | null>(null)

function isThemeId(value: string | null): value is ThemeId {
  return THEME_OPTIONS.some((option) => option.id === value)
}

function isAppearanceId(value: string | null): value is AppearanceId {
  return value === 'dark' || value === 'light'
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setTheme] = useState<ThemeId>(() => {
    const savedTheme = window.localStorage.getItem(STORAGE_KEY)
    return isThemeId(savedTheme) ? savedTheme : DEFAULT_THEME
  })
  const [appearance, setAppearance] = useState<AppearanceId>(() => {
    const savedAppearance = window.localStorage.getItem(APPEARANCE_STORAGE_KEY)
    return isAppearanceId(savedAppearance) ? savedAppearance : DEFAULT_APPEARANCE
  })

  useEffect(() => {
    document.documentElement.dataset.theme = theme
    window.localStorage.setItem(STORAGE_KEY, theme)
  }, [theme])

  useEffect(() => {
    document.documentElement.dataset.appearance = appearance
    document.documentElement.style.colorScheme = appearance
    window.localStorage.setItem(APPEARANCE_STORAGE_KEY, appearance)
  }, [appearance])

  const value = useMemo(
    () => ({ theme, setTheme, appearance, setAppearance }),
    [theme, appearance]
  )

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
}

export function useTheme() {
  const context = useContext(ThemeContext)
  if (!context) throw new Error('useTheme precisa estar dentro de ThemeProvider')
  return context
}
