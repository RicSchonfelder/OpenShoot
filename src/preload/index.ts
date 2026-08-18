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
  scanFolder: (dir: string): Promise<ScanResultData | { error: string }> => ipcRenderer.invoke('core:scanFolder', dir),
  listPhotos: (search: string, offset: number, limit: number): Promise<PhotoListData> =>
    ipcRenderer.invoke('core:listPhotos', search, offset, limit),
  photoCount: (): Promise<number> => ipcRenderer.invoke('core:photoCount'),
  thumbForPhoto: (id: number, maxDim: number): Promise<string | null> =>
    ipcRenderer.invoke('core:thumbForPhoto', id, maxDim),
  pickFolder: (): Promise<string | null> => ipcRenderer.invoke('core:pickFolder'),
  // Fase 2
  cullPhotos: (): Promise<{ processed: number; errors: number; avgScore: number; picks: number }> =>
    ipcRenderer.invoke('core:cullPhotos'),
  writeXmpForPhoto: (id: number): Promise<string> =>
    ipcRenderer.invoke('core:writeXmpForPhoto', id)
}

contextBridge.exposeInMainWorld('openshoot', api)

export type OpenShootApi = typeof api
