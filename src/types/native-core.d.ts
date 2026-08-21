// Declaração global do addon nativo (napi-rs). Deve ser um SCRIPT (sem export
// no topo) para que o `declare module '*.node'` seja global e visível em todo o projeto.
declare module '*.node' {
  export interface NativePhotoMeta {
    id: number
    path: string
    fileName: string
    ext: string
    fileSize: number
    width: number
    height: number
    camera: string
    takenAt: string | null
    rating: number
    hasXmp: boolean
    previewAvailable: boolean
    cullScore: number | null
    hash: string
  }
  export interface NativeScanResult {
    scanned: number
    added: number
    updated: number
    skipped: number
    errors: string[]
  }
  export interface NativePhotoList {
    photos: NativePhotoMeta[]
    total: number
  }
  export interface NativeCullSummary {
    processed: number
    errors: number
    avgScore: number
    picks: number
  }
  export function setup(dataDir: string): string
  export function add(a: number, b: number): number
  export function coreVersion(): string
  export function hello(name: string): string
  export function scanFolder(dir: string): NativeScanResult
  export function listPhotos(search: string, filter: string, offset: number, limit: number): NativePhotoList
  export function getPhoto(id: number): NativePhotoMeta | null
  export function photoCount(): number
  export function thumbForPhoto(id: number, maxDim: number): Promise<string | null>
  export function thumbForPath(path: string, maxDim: number): Promise<string | null>
  export function cullPhotos(): Promise<NativeCullSummary>
  export function writeXmpForPhoto(id: number): string
  export function exportAllXmp(): { exported: number; errors: number; total: number }
  export function detectFacesInPath(path: string): unknown
  export function setPhotoEdit(id: number, paramsJson: string): void
  export function getPhotoEdit(id: number): string
  export function previewEdit(id: number, paramsJson: string, maxDim: number): Promise<string | null>
  export function applyEditAll(paramsJson: string): string
  export function applyEditOne(id: number, paramsJson: string, maxDim: number): Promise<string | null>
  export function retouchSkinPhoto(id: number, intensity: number, maxDim: number): Promise<string | null>
  export function applyRetouch(id: number, intensity: number, maxDim: number): Promise<string | null>
  export function inpaintPhoto(id: number, maskRect: number[], maxDim: number): Promise<string | null>
  export function generateCaption(id: number): Promise<string>
  export function setRating(id: number, rating: number): void
  export function deletePhoto(id: number): boolean
  export function removePhotoFromCatalog(id: number): boolean
  export function findDuplicates(): Array<{ hash: string; photo_ids: number[]; photo_names: string[]; photo_paths: string[] }>
  export function scanFolderProgress(
    dir: string,
    onProgress: (p: { processed: number; total: number; currentFile: string }) => void
  ): Promise<string>
  export function clearThumbCache(): number
}
