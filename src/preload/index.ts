import { contextBridge, ipcRenderer } from 'electron'
import type { PhotoListData, PhotoMeta, ScanResultData } from '../types/photo'

export type { PhotoListData, PhotoMeta, ScanResultData }

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
  pickFolder: (): Promise<string | null> => ipcRenderer.invoke('core:pickFolder'),
  pickPresetFile: (): Promise<string | null> => ipcRenderer.invoke('core:pickPresetFile'),
  importLightroomPreset: (path: string, name?: string): Promise<{ ok: boolean; name?: string; recipe?: string; error?: string }> =>
    ipcRenderer.invoke('core:importLightroomPreset', path, name),
  setSessionType: (pathPrefix: string, sessionType: string): Promise<{ ok: boolean; updated?: number; error?: string }> =>
    ipcRenderer.invoke('core:setSessionType', pathPrefix, sessionType),
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
  listPresets: (): Promise<Array<{ name: string; recipe: string }>> =>
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
  scanFolderProgress: (
    dir: string,
    includeSubdirs: boolean,
    types: string,
    onProgress: (p: { processed: number; total: number; currentFile: string }) => void
  ): Promise<string> =>
    ipcRenderer.invoke('core:scanFolderProgress', dir, includeSubdirs, types, onProgress),
  clearThumbCache: (): Promise<number> => ipcRenderer.invoke('core:clearThumbCache')
}

contextBridge.exposeInMainWorld('openshoot', api)

export type OpenShootApi = typeof api
