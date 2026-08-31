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
    hasFace: boolean
    review: boolean
    aiPick: boolean
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
    review: number
  }
  export interface NativeExifDetail {
    iso: number | null
    aperture: number | null
    focal_length: number | null
    shutter_speed: string | null
    lens: string
    flash: string | null
    white_balance: string | null
  }
  export function setup(dataDir: string): string
  export function add(a: number, b: number): number
  export function coreVersion(): string
  export function hello(name: string): string
  export function scanFolder(dir: string, includeSubdirs?: boolean, types?: string): NativeScanResult
  export function listPhotos(search: string, filter: string, offset: number, limit: number): NativePhotoList
  export function getPhoto(id: number): NativePhotoMeta | null
  export function photoCount(): number
  export function thumbForPhoto(id: number, maxDim: number): Promise<string | null>
  export function thumbForPath(path: string, maxDim: number): Promise<string | null>
  export function cullPhotos(targetPicks?: number, photoIds?: number[] | null): Promise<NativeCullSummary>
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
  export function filterCounts(photoIds?: number[] | null): { all: number; picks: number; rejects: number; unrated: number; review: number; destaques: number; selecionado: number; duplicates: number; faces: number; edited: number }
  export function savePreset(name: string, recipe: string): void
  export function savePresetFull(name: string, recipe: string, fileType: string, colorType: string, source: string): void
  export function updatePresetMeta(name: string, fileType: string, colorType: string): boolean
  export function listPresets(): Array<{ name: string; recipe: string; file_type: string; color_type: string; source: string }>
  export function deletePreset(name: string): boolean
  export function autoLevelPhoto(id: number, maxDim: number): Promise<{ preview: string; angle: number } | { error: string }>
  export function aiCropPhoto(id: number, maxDim: number): Promise<string | null>
  export function retouchFacePhoto(id: number, region: string, intensity: number, maxDim: number): Promise<string | null>
  export function learnProfile(): { ok: boolean; name?: string; photos?: number; error?: string }
  export function importLightroomPreset(path: string, name?: string): { ok: boolean; name?: string; recipe?: string; error?: string }
  export function setSessionType(pathPrefix: string, sessionType: string): { ok: boolean; updated?: number; error?: string }
  export function detectFacesInPhoto(id: number): { count: number; faces: number[][]; width: number; height: number }
  export function getExifDetail(id: number): NativeExifDetail
  export function exportPresetToFile(name: string, dest: string): { ok: boolean; name?: string; error?: string }
  export function importPresetFromFile(path: string): { ok: boolean; name?: string; error?: string }
  export function subjectMaskPhoto(id: number, blur: number, maxDim: number): Promise<string | null>
  export function createAlbum(name: string): number
  export function listAlbums(): Array<{ id: number; name: string; session_type: string; cover_photo_id: number | null; created_at: string; photo_count: number; cover_path: string | null }>
  export function deleteAlbum(id: number): boolean
  export function addPhotosToAlbum(albumId: number, photoIds: number[]): number
  export function addFolderToAlbum(albumId: number, dir: string): number
  export function setAlbumSessionType(albumId: number, sessionType: string): void
  export function albumPhotoIds(albumId: number): number[]
  export function exportPhotos(ids: number[], destDir: string, format: string, quality: number, colorProfile: string, naming: string): { ok: boolean; exported?: number; errors?: number; files?: string[]; dest_dir?: string; error?: string }
  export function applyRetouchAll(ids: number[], destDir: string, skin: number, regions: Record<string, number>, format: string, quality: number): { ok: boolean; exported?: number; errors?: number; files?: string[]; dest_dir?: string; error?: string }
  export function createWebGallery(ids: number[], destDir: string, title: string): { ok: boolean; path?: string; count?: number; error?: string }
  export function scanFolderProgress(
    dir: string,
    includeSubdirs: boolean,
    types: string,
    onProgress: (p: { processed: number; total: number; currentFile: string }) => void
  ): Promise<string>
  export function clearThumbCache(): number
  export function upscaleAvailable(model?: string): boolean
  export function upscalePhoto(id: number, model?: string, scale?: number, maxDim?: number): Promise<string | null>
  export function exportUpscaled(
    ids: number[],
    destDir: string,
    model?: string,
    scale?: number,
    format?: string,
    quality?: number,
    onProgress?: (p: { processed: number; total: number; currentFile: string }) => void
  ): Promise<string>
}
