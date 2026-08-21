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
    // fallback: se o userData nao tiver permissao, tenta diretorio padrao do app
    try {
      core.setup(join(app.getAppPath(), '.data'))
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
  ipcMain.handle('core:scanFolder', async (_e, dir: string) => {
    try {
      return getCore().scanFolder(dir)
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
    } catch {
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

  // ---- Fase 2: culling + XMP ----
  ipcMain.handle('core:cullPhotos', async () => {
    try {
      return await getCore().cullPhotos()
    } catch (e) {
      return { processed: 0, errors: 1, avgScore: 0, picks: 0, error: String(e) }
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
  ipcMain.handle(
    'core:scanFolderProgress',
    async (_event, dir: string, onProgress: (p: any) => void) => {
      try {
        return await getCore().scanFolderProgress(dir, onProgress)
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

  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
