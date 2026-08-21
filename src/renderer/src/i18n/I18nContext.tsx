import { createContext, useCallback, useContext, useMemo, useState } from 'react'
import type { ReactNode } from 'react'
import { dictionaries, detectLocale, interpolate } from './index'
import type { Locale } from './index'

interface I18nContextValue {
  locale: Locale
  setLocale: (l: Locale) => void
  t: (key: string, params?: Record<string, string | number>) => string
}

const I18nContext = createContext<I18nContextValue>({
  locale: 'en',
  setLocale: () => {},
  t: (key) => key
})

export function I18nProvider({ children }: { children: ReactNode }) {
  const [locale, setLocale] = useState<Locale>(detectLocale)
  const t = useCallback(
    (key: string, params?: Record<string, string | number>) => {
      const dict = dictionaries[locale]
      const template = dict[key] ?? key
      return interpolate(template, params)
    },
    [locale]
  )
  const value = useMemo(() => ({ locale, setLocale, t }), [locale, t])
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useT() {
  return useContext(I18nContext)
}