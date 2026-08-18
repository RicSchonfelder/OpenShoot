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

  createWindow()

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') app.quit()
})
