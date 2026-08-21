/**
 * Cache em memória de thumbnails (base64) por photoId + maxDim.
 *
 * Resolve o problema de "previews somem de repente" ao recarregar a lista:
 * os thumbnails já carregados são reutilizados instantaneamente em vez de
 * serem re-decodificados do zero.
 *
 * Limite simples para não estourar memória (libera os menos recentes).
 */

interface CacheEntry {
  src: string
  lastAccess: number
}

const cache = new Map<number, CacheEntry>()
const MAX_ENTRIES = 2000

export function getThumb(id: number): string | null {
  const e = cache.get(id)
  if (e) {
    e.lastAccess = Date.now()
    return e.src
  }
  return null
}

export function setThumb(id: number, src: string): void {
  // Evita re-adicionar se já existe
  if (cache.has(id)) return
  cache.set(id, { src, lastAccess: Date.now() })
  // Evita estouro: remove as entradas mais antigas quando passa do limite.
  if (cache.size > MAX_ENTRIES) {
    const entries = Array.from(cache.entries()).sort((a, b) => a[1].lastAccess - b[1].lastAccess)
    const toRemove = entries.slice(0, cache.size - MAX_ENTRIES)
    for (const [k] of toRemove) cache.delete(k)
  }
}

export function clearThumbCache(): void {
  cache.clear()
}

export function thumbCacheSize(): number {
  return cache.size
}
