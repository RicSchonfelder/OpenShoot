import { app, BrowserWindow, dialog, ipcMain, shell } from 'electron'
import { join } from 'node:path'
import { loadCore, getCore } from './core'

function createWindow(): void {
  const win = new BrowserWindow({
    width: 1280,
    height: 860,
    minWidth: 800,
    minHeight: 600,
    title: 'OpenShoot',
    backgroundColor: '#0f1115',
    show: false,
    webPreferences: {
      preload: join(__dirname, '../preload/index.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false
    }
  })

  win.on('ready-to-show', () => win.show())

  win.webContents.setWindowOpenHandler((details) => {
    shell.openExternal(details.url)
    return { action: 'deny' }
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    win.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    win.loadFile(join(__dirname, '../renderer/index.html'))
  }
}

app.whenReady().then(() => {
  loadCore()

  // Inicializa o catálogo SQLite no diretório de dados do usuário.
  const core = getCore()
  try {
    core.setup(app.getPath('userData'))
  } catch (e) {
    // fallback multi-plataforma: home do usuário (Program Files bloqueia escrita)
    try {
      core.setup(join(app.getPath('home'), '.openshoot-data'))
    } catch (e2) {
      console.error('Falha ao inicializar catálogo:', e2)
    }
  }

  ipcMain.handle('core:hello', (_e, name: string) => getCore().hello(name))
  ipcMain.handle('core:add', (_e, a: number, b: number) => getCore().add(a, b))
  ipcMain.handle('core:version', () => getCore().coreVersion())
  ipcMain.handle('app:info', () => ({
    platform: process.platform,
    arch: process.arch,
    versions: {
      electron: process.versions.electron,
      node: process.versions.node,
      chrome: process.versions.chrome
    }
  }))

  // ---- Fase 1: catálogo + thumbnails ----
  ipcMain.handle('core:scanFolder', async (_e, dir: string, includeSubdirs?: boolean, types?: string) => {
    try {
      return getCore().scanFolder(dir, includeSubdirs, types)
    } catch (e) {
      return { error: String(e) }
    }
  })
  ipcMain.handle(
    'core:listPhotos',
    (_e, search: string, filter: string, offset: number, limit: number) =>
      getCore().listPhotos(search, filter, offset, limit)
  )
  ipcMain.handle('core:photoCount', () => getCore().photoCount())
  ipcMain.handle('core:thumbForPhoto', async (_e, id: number, maxDim: number) => {
    try {
      return (await getCore().thumbForPhoto(id, maxDim)) ?? null
    } catch (e) {
      return null
    }
  })
  ipcMain.handle('core:thumbForPath', async (_e, path: string, maxDim: number) => {
    try {
      return (await getCore().thumbForPath(path, maxDim)) ?? null
    } catch (e) {
      return null
    }
  })

  ipcMain.handle('core:pickFolder', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await dialog.showOpenDialog(win, {
      title: 'Escolha uma pasta de fotos',
      properties: ['openDirectory']
    })
    if (result.canceled || !result.filePaths.length) return null
    return result.filePaths[0]
  })
  ipcMain.handle('core:pickPresetFile', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await dialog.showOpenDialog(win, {
      title: 'Escolha um preset do Lightroom',
      properties: ['openFile'],
      filters: [
        { name: 'Presets Lightroom', extensions: ['xmp', 'lrtemplate'] },
        { name: 'Todos', extensions: ['*'] }
      ]
    })
    if (result.canceled || !result.filePaths.length) return null
    return result.filePaths[0]
  })
  ipcMain.handle('core:pickPresetJson', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await dialog.showOpenDialog(win, {
      title: 'Escolha um preset OpenShoot (.json)',
      properties: ['openFile'],
      filters: [{ name: 'Presets OpenShoot', extensions: ['json'] }]
    })
    if (result.canceled || !result.filePaths.length) return null
    return result.filePaths[0]
  })
  ipcMain.handle('core:savePresetAs', async (event, _defaultName: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await dialog.showSaveDialog(win, {
      title: 'Exportar preset',
      defaultPath: `${_defaultName || 'preset'}.json`,
      filters: [{ name: 'Presets OpenShoot', extensions: ['json'] }]
    })
    if (result.canceled || !result.filePath) return null
    return result.filePath
  })

  // ---- Fase 2: culling + XMP ----
  ipcMain.handle('core:cullPhotos', async (_e, targetPicks?: number) => {
    try {
      return await getCore().cullPhotos(targetPicks)
    } catch (e) {
      return { processed: 0, errors: 1, avgScore: 0, picks: 0, review: 0, error: String(e) }
    }
  })
  ipcMain.handle('core:writeXmpForPhoto', (_e, id: number) => {
    try {
      return getCore().writeXmpForPhoto(id)
    } catch (e) {
      return String(e)
    }
  })
  ipcMain.handle('core:exportAllXmp', async () => {
    try {
      return await getCore().exportAllXmp()
    } catch (e) {
      return { exported: 0, errors: 1, total: 0, error: String(e) }
    }
  })

  // ---- Fase 3: edição ----
  ipcMain.handle('core:setPhotoEdit', (_e, id: number, paramsJson: string) => {
    try {
      return getCore().setPhotoEdit(id, paramsJson)
    } catch (e) {
      return { error: String(e) }
    }
  })
  ipcMain.handle('core:getPhotoEdit', (_e, id: number) => getCore().getPhotoEdit(id))
  ipcMain.handle('core:previewEdit', async (_e, id: number, paramsJson: string, maxDim: number) => {
    try {
      return (await getCore().previewEdit(id, paramsJson, maxDim)) ?? null
    } catch {
      return null
    }
  })
  ipcMain.handle('core:applyEditAll', async (_e, paramsJson: string) => {
    try {
      return await getCore().applyEditAll(paramsJson)
    } catch (e) {
      return JSON.stringify({ applied: 0, errors: 1, total: 0 })
    }
  })
  ipcMain.handle('core:applyEditOne', async (_e, id: number, paramsJson: string, maxDim: number) => {
    try {
      return (await getCore().applyEditOne(id, paramsJson, maxDim)) ?? null
    } catch {
      return null
    }
  })

  // ---- Fase 4: retoque ----
  ipcMain.handle('core:retouchSkinPhoto', async (_e, id: number, intensity: number, maxDim: number) => {
    try {
      return (await getCore().retouchSkinPhoto(id, intensity, maxDim)) ?? null
    } catch {
      return null
    }
  })
  ipcMain.handle('core:applyRetouch', async (_e, id: number, intensity: number, maxDim: number) => {
    try {
      return (await getCore().applyRetouch(id, intensity, maxDim)) ?? null
    } catch {
      return null
    }
  })
  ipcMain.handle('core:inpaintPhoto', async (_e, id: number, maskRect: number[], maxDim: number) => {
    try {
      return (await getCore().inpaintPhoto(id, maskRect, maxDim)) ?? null
    } catch {
      return null
    }
  })

  // ---- Fase 6 + UX ----
  ipcMain.handle('core:generateCaption', async (_e, id: number) => {
    try {
      return await getCore().generateCaption(id)
    } catch (e) {
      return JSON.stringify({ error: String(e) })
    }
  })
  ipcMain.handle('core:setRating', (_e, id: number, rating: number) => {
    try {
      return getCore().setRating(id, rating)
    } catch (e) {
      return { error: String(e) }
    }
  })
  ipcMain.handle('core:deletePhoto', async (_e, id: number) => {
    try {
      return await getCore().deletePhoto(id)
    } catch (e) {
      return false
    }
  })
  ipcMain.handle('core:removePhotoFromCatalog', async (_e, id: number) => {
    try {
      return await getCore().removePhotoFromCatalog(id)
    } catch (e) {
      return false
    }
  })
  ipcMain.handle('core:findDuplicates', async () => {
    try {
      return await getCore().findDuplicates()
    } catch (e) {
      return []
    }
  })
  ipcMain.handle('core:filterCounts', async () => {
    try {
      return await getCore().filterCounts()
    } catch (e) {
      return null
    }
  })
  ipcMain.handle('core:savePreset', async (_e, name: string, recipe: string) => {
    try {
      await getCore().savePreset(name, recipe)
      return true
    } catch (e) {
      return false
    }
  })
  ipcMain.handle(
    'core:savePresetFull',
    async (_e, name: string, recipe: string, fileType: string, colorType: string, source: string) => {
      try {
        await getCore().savePresetFull(name, recipe, fileType, colorType, source)
        return true
      } catch (e) {
        return false
      }
    }
  )
  ipcMain.handle('core:listPresets', async () => {
    try {
      return await getCore().listPresets()
    } catch (e) {
      return []
    }
  })
  ipcMain.handle('core:deletePreset', async (_e, name: string) => {
    try {
      return await getCore().deletePreset(name)
    } catch (e) {
      return false
    }
  })
  ipcMain.handle('core:autoLevelPhoto', async (_e, id: number, maxDim: number) => {
    try {
      return await getCore().autoLevelPhoto(id, maxDim)
    } catch (e) {
      return { error: String(e) }
    }
  })
  ipcMain.handle('core:aiCropPhoto', async (_e, id: number, maxDim: number) => {
    try {
      return await getCore().aiCropPhoto(id, maxDim)
    } catch (e) {
      return null
    }
  })
  ipcMain.handle('core:retouchFacePhoto', async (_e, id: number, region: string, intensity: number, maxDim: number) => {
    try {
      return await getCore().retouchFacePhoto(id, region, intensity, maxDim)
    } catch (e) {
      return null
    }
  })
  ipcMain.handle('core:learnProfile', async () => {
    try {
      return await getCore().learnProfile()
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle('core:importLightroomPreset', async (_e, path: string, name?: string) => {
    try {
      return await getCore().importLightroomPreset(path, name)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle('core:setSessionType', async (_e, pathPrefix: string, sessionType: string) => {
    try {
      return await getCore().setSessionType(pathPrefix, sessionType)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle('core:detectFacesInPhoto', async (_e, id: number) => {
    try {
      return await getCore().detectFacesInPhoto(id)
    } catch (e) {
      return { count: 0, faces: [] }
    }
  })
  ipcMain.handle('core:exportPresetToFile', async (_e, name: string, dest: string) => {
    try {
      return await getCore().exportPresetToFile(name, dest)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle('core:importPresetFromFile', async (_e, path: string) => {
    try {
      return await getCore().importPresetFromFile(path)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle('core:subjectMaskPhoto', async (_e, id: number, blur: number, maxDim: number) => {
    try {
      return await getCore().subjectMaskPhoto(id, blur, maxDim)
    } catch (e) {
      return null
    }
  })
  ipcMain.handle('core:createAlbum', async (_e, name: string) => {
    try {
      return await getCore().createAlbum(name)
    } catch (e) {
      return -1
    }
  })
  ipcMain.handle('core:listAlbums', async () => {
    try {
      return await getCore().listAlbums()
    } catch (e) {
      return []
    }
  })
  ipcMain.handle('core:deleteAlbum', async (_e, id: number) => {
    try {
      return await getCore().deleteAlbum(id)
    } catch (e) {
      return false
    }
  })
  ipcMain.handle('core:addPhotosToAlbum', async (_e, albumId: number, photoIds: number[]) => {
    try {
      return await getCore().addPhotosToAlbum(albumId, photoIds)
    } catch (e) {
      return 0
    }
  })
  ipcMain.handle('core:addFolderToAlbum', async (_e, albumId: number, dir: string) => {
    try {
      return await getCore().addFolderToAlbum(albumId, dir)
    } catch (e) {
      return 0
    }
  })
  ipcMain.handle('core:setAlbumSessionType', async (_e, albumId: number, sessionType: string) => {
    try {
      return await getCore().setAlbumSessionType(albumId, sessionType)
    } catch (e) {
      return false
    }
  })
  ipcMain.handle('core:albumPhotoIds', async (_e, albumId: number) => {
    try {
      return await getCore().albumPhotoIds(albumId)
    } catch (e) {
      return []
    }
  })
  ipcMain.handle('core:exportPhotos', async (_e, ids: number[], destDir: string, format: string, quality: number, colorProfile: string, naming: string) => {
    try {
      return await getCore().exportPhotos(ids, destDir, format, quality, colorProfile, naming)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle('core:pickExportFolder', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return null
    const result = await dialog.showOpenDialog(win, {
      title: 'Escolha a pasta de exportação',
      properties: ['openDirectory', 'createDirectory']
    })
    if (result.canceled || !result.filePaths.length) return null
    return result.filePaths[0]
  })
  ipcMain.handle('core:applyRetouchAll', async (_e, ids: number[], destDir: string, skin: number, regions: any, format: string, quality: number) => {
    try {
      return await getCore().applyRetouchAll(ids, destDir, skin, regions, format, quality)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  // --- Galeria web (agent-05) ---
  ipcMain.handle('core:createWebGallery', async (_e, ids: number[], destDir: string, title: string) => {
    try {
      return await getCore().createWebGallery(ids, destDir, title)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })
  ipcMain.handle(
    'core:scanFolderProgress',
    async (
      _event,
      dir: string,
      includeSubdirs: boolean,
      types: string,
      onProgress: (p: any) => void
    ) => {
      try {
        return await getCore().scanFolderProgress(dir, includeSubdirs, types, onProgress)
      } catch (e) {
        return JSON.stringify({ error: String(e), scanned: 0, added: 0, updated: 0 })
      }
    }
  )
  ipcMain.handle('core:clearThumbCache', () => {
    try {
      return getCore().clearThumbCache()
    } catch (e) {
      return 0
    }
  })

  // ---- Pessoas: agrupamento facial + exportação por pessoa ----
  type CoreWithPeople = typeof import('*.node') & {
    groupBySimilarityAsync(threshold: number | null | undefined): Promise<unknown>
    exportPeopleToFolders(outDir: string, threshold: number | null | undefined): Promise<unknown>
  }
  ipcMain.handle('core:groupBySimilarity', async (_e, threshold?: number) => {
    try {
      return await (getCore() as CoreWithPeople).groupBySimilarityAsync(threshold ?? null)
    } catch (e) {
      return { error: String(e) }
    }
  })
  ipcMain.handle('core:exportPeopleToFolders', async (_e, outDir: string, threshold?: number) => {
    try {
      return await (getCore() as CoreWithPeople).exportPeopleToFolders(outDir, threshold ?? null)
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })

  // --- Labels de cor (agent-10) ---
  type CoreWithLabels = typeof import('*.node') & {
    setPhotoLabel(id: number, label: string): void
    getPhotoLabel(id: number): string
    getLabelsBulk(ids: number[]): Record<string, string>
  }
  ipcMain.handle('core:setPhotoLabel', (_e, id: number, label: string) => {
    try {
      ;(getCore() as CoreWithLabels).setPhotoLabel(id, label)
      return true
    } catch (err) {
      console.error('Falha ao definir etiqueta:', err)
      return false
    }
  })
  ipcMain.handle('core:getPhotoLabel', (_e, id: number) => {
    try {
      return (getCore() as CoreWithLabels).getPhotoLabel(id)
    } catch (err) {
      console.error('Falha ao ler etiqueta:', err)
      return ''
    }
  })
  ipcMain.handle('core:getLabelsBulk', (_e, ids: number[]) => {
    try {
      return (getCore() as CoreWithLabels).getLabelsBulk(ids)
    } catch (err) {
      console.error('Falha ao ler etiquetas (lote):', err)
      return {}
    }
  })

  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
