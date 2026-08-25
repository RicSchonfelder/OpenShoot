import { contextBridge, ipcRenderer } from 'electron'
import type { PhotoListData, PhotoMeta, ScanResultData } from '../types/photo'

export type { PhotoListData, PhotoMeta, ScanResultData }

export interface PersonGroup {
  person_id: number
  count: number
  sample_path: string
  photo_ids: number[]
  photo_paths: string[]
}

export interface PeopleExportResult {
  ok: boolean
  out_dir?: string
  groups?: Array<{ person_id: number; folder: string; count: number; sample: string }>
  exported?: number
  no_face?: number
  error?: string
}

export interface ImportProgress {
  processed: number
  total: number
  currentFile?: string
}

const api = {
  hello: (name: string): Promise<string> => ipcRenderer.invoke('core:hello', name),
  add: (a: number, b: number): Promise<number> => ipcRenderer.invoke('core:add', a, b),
  coreVersion: (): Promise<string> => ipcRenderer.invoke('core:version'),
  appInfo: (): Promise<{ platform: string; arch: string; versions: Record<string, string | undefined> }> =>
    ipcRenderer.invoke('app:info'),
  // Fase 1
  scanFolder: (dir: string, includeSubdirs?: boolean, types?: string): Promise<ScanResultData | { error: string }> =>
    ipcRenderer.invoke('core:scanFolder', dir, includeSubdirs, types),
  listPhotos: (search: string, filter: string, offset: number, limit: number): Promise<PhotoListData> =>
    ipcRenderer.invoke('core:listPhotos', search, filter, offset, limit),
  photoCount: (): Promise<number> => ipcRenderer.invoke('core:photoCount'),
  thumbForPhoto: (id: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:thumbForPhoto', id, maxDim),
  thumbForPath: (path: string, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:thumbForPath', path, maxDim),
  pickFolder: (): Promise<string | null> => ipcRenderer.invoke('core:pickFolder'),
  pickPresetFile: (): Promise<string | null> => ipcRenderer.invoke('core:pickPresetFile'),
  importLightroomPreset: (path: string, name?: string): Promise<{ ok: boolean; name?: string; recipe?: string; error?: string }> =>
    ipcRenderer.invoke('core:importLightroomPreset', path, name),
  setSessionType: (pathPrefix: string, sessionType: string): Promise<{ ok: boolean; updated?: number; error?: string }> =>
    ipcRenderer.invoke('core:setSessionType', pathPrefix, sessionType),
  detectFacesInPhoto: (id: number): Promise<{ count: number; faces: number[][]; width: number; height: number }> =>
    ipcRenderer.invoke('core:detectFacesInPhoto', id),
  pickPresetJson: (): Promise<string | null> => ipcRenderer.invoke('core:pickPresetJson'),
  savePresetAs: (defaultName: string): Promise<string | null> => ipcRenderer.invoke('core:savePresetAs', defaultName),
  exportPresetToFile: (name: string, dest: string): Promise<{ ok: boolean; name?: string; error?: string }> =>
    ipcRenderer.invoke('core:exportPresetToFile', name, dest),
  importPresetFromFile: (path: string): Promise<{ ok: boolean; name?: string; error?: string }> =>
    ipcRenderer.invoke('core:importPresetFromFile', path),
  subjectMaskPhoto: (id: number, blur: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:subjectMaskPhoto', id, blur, maxDim),
  createAlbum: (name: string): Promise<number> => ipcRenderer.invoke('core:createAlbum', name),
  listAlbums: (): Promise<Array<{ id: number; name: string; sessionType: string; coverPhotoId: number | null; createdAt: string; photoCount: number; coverPath: string | null }>> =>
    ipcRenderer.invoke('core:listAlbums'),
  deleteAlbum: (id: number): Promise<boolean> => ipcRenderer.invoke('core:deleteAlbum', id),
  addPhotosToAlbum: (albumId: number, photoIds: number[]): Promise<number> =>
    ipcRenderer.invoke('core:addPhotosToAlbum', albumId, photoIds),
  addFolderToAlbum: (albumId: number, dir: string): Promise<number> =>
    ipcRenderer.invoke('core:addFolderToAlbum', albumId, dir),
  setAlbumSessionType: (albumId: number, sessionType: string): Promise<boolean> =>
    ipcRenderer.invoke('core:setAlbumSessionType', albumId, sessionType),
  albumPhotoIds: (albumId: number): Promise<number[]> => ipcRenderer.invoke('core:albumPhotoIds', albumId),
  exportPhotos: (ids: number[], destDir: string, format: string, quality: number, colorProfile: string, naming: string): Promise<{ ok: boolean; exported?: number; errors?: number; files?: string[]; dest_dir?: string; error?: string }> =>
    ipcRenderer.invoke('core:exportPhotos', ids, destDir, format, quality, colorProfile, naming),
  pickExportFolder: (): Promise<string | null> => ipcRenderer.invoke('core:pickExportFolder'),
  applyRetouchAll: (ids: number[], destDir: string, skin: number, regions: Record<string, number>, format: string, quality: number): Promise<{ ok: boolean; exported?: number; errors?: number; files?: string[]; dest_dir?: string; error?: string }> =>
    ipcRenderer.invoke('core:applyRetouchAll', ids, destDir, skin, regions, format, quality),
  createWebGallery: (ids: number[], destDir: string, title: string): Promise<{ ok: boolean; path?: string; count?: number; error?: string }> =>
    ipcRenderer.invoke('core:createWebGallery', ids, destDir, title),
  // Fase 2
  cullPhotos: (targetPicks?: number): Promise<{ processed: number; errors: number; avgScore: number; picks: number; review: number }> =>
    ipcRenderer.invoke('core:cullPhotos', targetPicks),
  writeXmpForPhoto: (id: number): Promise<string> =>
    ipcRenderer.invoke('core:writeXmpForPhoto', id),
  exportAllXmp: (): Promise<{ exported: number; errors: number; total: number }> =>
    ipcRenderer.invoke('core:exportAllXmp'),
  // Fase 3: edição
  setPhotoEdit: (id: number, paramsJson: string): Promise<void> =>
    ipcRenderer.invoke('core:setPhotoEdit', id, paramsJson),
  getPhotoEdit: (id: number): Promise<string> =>
    ipcRenderer.invoke('core:getPhotoEdit', id),
  previewEdit: (id: number, paramsJson: string, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:previewEdit', id, paramsJson, maxDim),
  applyEditAll: (paramsJson: string): Promise<string> =>
    ipcRenderer.invoke('core:applyEditAll', paramsJson),
  applyEditOne: (id: number, paramsJson: string, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:applyEditOne', id, paramsJson, maxDim),
  // Fase 4: retoque
  retouchSkinPhoto: (id: number, intensity: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:retouchSkinPhoto', id, intensity, maxDim),
  applyRetouch: (id: number, intensity: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:applyRetouch', id, intensity, maxDim),
  inpaintPhoto: (id: number, maskRect: number[], maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:inpaintPhoto', id, maskRect, maxDim),
  // Fase 6 + UX
  generateCaption: (id: number): Promise<string> =>
    ipcRenderer.invoke('core:generateCaption', id),
  setRating: (id: number, rating: number): Promise<void> =>
    ipcRenderer.invoke('core:setRating', id, rating),
  deletePhoto: (id: number): Promise<boolean> =>
    ipcRenderer.invoke('core:deletePhoto', id),
  removePhotoFromCatalog: (id: number): Promise<boolean> =>
    ipcRenderer.invoke('core:removePhotoFromCatalog', id),
  findDuplicates: (): Promise<Array<{ hash: string; photo_ids: number[]; photo_names: string[]; photo_paths: string[] }>> =>
    ipcRenderer.invoke('core:findDuplicates'),
  filterCounts: (): Promise<{ all: number; picks: number; rejects: number; unrated: number; review: number; destaques: number; selecionado: number; duplicates: number; faces: number; edited: number } | null> =>
    ipcRenderer.invoke('core:filterCounts'),
  savePreset: (name: string, recipe: string): Promise<boolean> =>
    ipcRenderer.invoke('core:savePreset', name, recipe),
  savePresetFull: (
    name: string,
    recipe: string,
    fileType: string,
    colorType: string,
    source: string
  ): Promise<boolean> =>
    ipcRenderer.invoke('core:savePresetFull', name, recipe, fileType, colorType, source),
  listPresets: (): Promise<Array<{ name: string; recipe: string; file_type: string; color_type: string; source: string }>> =>
    ipcRenderer.invoke('core:listPresets'),
  deletePreset: (name: string): Promise<boolean> =>
    ipcRenderer.invoke('core:deletePreset', name),
  autoLevelPhoto: (id: number, maxDim: number): Promise<{ preview: string; angle: number } | { error: string }> =>
    ipcRenderer.invoke('core:autoLevelPhoto', id, maxDim),
  aiCropPhoto: (id: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:aiCropPhoto', id, maxDim),
  retouchFacePhoto: (id: number, region: string, intensity: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:retouchFacePhoto', id, region, intensity, maxDim),
  learnProfile: (): Promise<{ ok: boolean; name?: string; photos?: number; error?: string }> =>
    ipcRenderer.invoke('core:learnProfile'),
  scanFolderProgress: (dir: string, includeSubdirs: boolean, types: string): Promise<string> =>
    ipcRenderer.invoke('core:scanFolderProgress', dir, includeSubdirs, types),
  onImportProgress: (cb: (p: ImportProgress) => void): (() => void) => {
    const listener = (_e: Electron.IpcRendererEvent, p: ImportProgress): void => cb(p)
    ipcRenderer.on('core:import-progress', listener)
    return () => ipcRenderer.removeListener('core:import-progress', listener)
  },
  clearThumbCache: (): Promise<number> => ipcRenderer.invoke('core:clearThumbCache'),
  // Pessoas (agrupamento facial)
  groupBySimilarity: (
    threshold?: number
  ): Promise<PersonGroup[] | { groups?: PersonGroup[] } | { error: string }> =>
    ipcRenderer.invoke('core:groupBySimilarity', threshold),
  exportPeopleToFolders: (outDir: string, threshold?: number): Promise<PeopleExportResult> =>
    ipcRenderer.invoke('core:exportPeopleToFolders', outDir, threshold),
  // Labels de cor (agent-10)
  setPhotoLabel: (id: number, label: string): Promise<void> =>
    ipcRenderer.invoke('core:setPhotoLabel', id, label),
  getPhotoLabel: (id: number): Promise<string> => ipcRenderer.invoke('core:getPhotoLabel', id),
  getLabelsBulk: (ids: number[]): Promise<Record<string, string>> =>
    ipcRenderer.invoke('core:getLabelsBulk', ids)
}

contextBridge.exposeInMainWorld('openshoot', api)

export type OpenShootApi = typeof api
