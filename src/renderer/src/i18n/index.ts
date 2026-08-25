import ptBR from './pt-BR.json'
import en from './en.json'

export type Locale = 'pt-BR' | 'en'

export const dictionaries: Record<Locale, Record<string, string>> = {
  'pt-BR': ptBR,
  en
}

export function detectLocale(): Locale {
  const lang = (navigator.language || 'en').toLowerCase()
  return lang.startsWith('pt') ? 'pt-BR' : 'en'
}

export function interpolate(template: string, params?: Record<string, string | number>): string {
  if (!params) return template
  return template.replace(/\{(\w+)\}/g, (_, key) => {
    const v = params[key]
    return v != null ? String(v) : `{${key}}`
  })
}