import { app, BrowserWindow, dialog, ipcMain, safeStorage, shell } from 'electron'
import { existsSync } from 'node:fs'
import { appendFile, mkdir, readFile, rename, stat, writeFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
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
  // Modelos ONNX: aponta o core Rust para core/models antes de carregar o addon.
  // No app empacotado os modelos ficam em app.asar.unpacked (asarUnpack).
  const modelsCandidates = [
    join(app.getAppPath(), 'core/models'),
    join(app.getAppPath().replace('app.asar', 'app.asar.unpacked'), 'core/models')
  ]
  const modelsDir = modelsCandidates.find((p) => existsSync(p))
  if (modelsDir) process.env.OPENSHOOT_MODELS_DIR = modelsDir

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

  ipcMain.handle('app:saveRestoredPreview', async (event, dataUrl: string, defaultName: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win || typeof dataUrl !== 'string' || !dataUrl.startsWith('data:image/')) {
      return { ok: false, error: 'Prévia inválida.' }
    }
    const result = await dialog.showSaveDialog(win, {
      title: 'Salvar cópia restaurada',
      defaultPath: `${(defaultName || 'foto-restaurada').replace(/[<>:"/\\|?*\x00-\x1F]/g, '_')}.jpg`,
      filters: [{ name: 'Imagem JPEG', extensions: ['jpg', 'jpeg'] }]
    })
    if (result.canceled || !result.filePath) return { ok: false }
    const comma = dataUrl.indexOf(',')
    if (comma < 0) return { ok: false, error: 'Formato de prévia inválido.' }
    try {
      await writeFile(result.filePath, Buffer.from(dataUrl.slice(comma + 1), 'base64'))
      return { ok: true, path: result.filePath }
    } catch (e) {
      return { ok: false, error: String(e) }
    }
  })

  ipcMain.handle('app:saveRestoredPreviews', async (event, items: Array<{ dataUrl: string; defaultName: string }>) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win || !Array.isArray(items) || !items.length) return { ok: false, error: 'Nenhuma prévia para salvar.' }
    const result = await dialog.showOpenDialog(win, {
      title: 'Escolha a pasta para as fotos restauradas',
      properties: ['openDirectory', 'createDirectory']
    })
    if (result.canceled || !result.filePaths.length) return { ok: false }
    const outputDir = result.filePaths[0]
    let saved = 0
    try {
      for (const item of items) {
        if (typeof item?.dataUrl !== 'string' || !item.dataUrl.startsWith('data:image/')) continue
        const comma = item.dataUrl.indexOf(',')
        if (comma < 0) continue
        const name = `${(item.defaultName || 'foto-restaurada').replace(/[<>:"/\\|?*\x00-\x1F]/g, '_')}.jpg`
        await writeFile(join(outputDir, name), Buffer.from(item.dataUrl.slice(comma + 1), 'base64'))
        saved += 1
      }
      return { ok: true, path: outputDir, saved }
    } catch (e) {
      return { ok: false, error: String(e), path: outputDir, saved }
    }
  })

  const openAiKeyPath = () => join(app.getPath('userData'), 'openai-api-key.bin')
  const restorationCacheDir = () => join(app.getPath('userData'), 'restoration-cache')
  const restorationCachePath = (sourcePath: string) => join(restorationCacheDir(), `${createHash('sha256').update(sourcePath).digest('hex')}.json`)
  const persistRestorationCache = async (sourcePath: string, dataUrl: string) => {
    const source = await stat(sourcePath)
    await mkdir(restorationCacheDir(), { recursive: true })
    const cachePath = restorationCachePath(sourcePath)
    const tempPath = `${cachePath}.${process.pid}.tmp`
    await writeFile(tempPath, JSON.stringify({ sourcePath, sourceSize: source.size, sourceMtimeMs: source.mtimeMs, savedAt: new Date().toISOString(), dataUrl }), { encoding: 'utf8', mode: 0o600 })
    await rename(tempPath, cachePath)
  }
  const cloudConfirmations = new Map<number, number>()
  const readOpenAiKey = async (): Promise<string | null> => {
    const environmentKey = process.env.OPENAI_API_KEY?.trim()
    if (environmentKey?.startsWith('sk-')) return environmentKey
    if (!safeStorage.isEncryptionAvailable()) return null
    try {
      return safeStorage.decryptString(await readFile(openAiKeyPath()))
    } catch {
      return null
    }
  }

  ipcMain.handle('app:hasOpenAiKey', async () => Boolean(await readOpenAiKey()))
  ipcMain.handle('app:saveOpenAiKey', async (_event, key: string) => {
    if (!safeStorage.isEncryptionAvailable()) return { ok: false, error: 'Armazenamento seguro indisponível nesta sessão.' }
    const normalized = typeof key === 'string' ? key.trim() : ''
    if (!normalized.startsWith('sk-')) return { ok: false, error: 'A chave deve começar com sk-.' }
    await writeFile(openAiKeyPath(), safeStorage.encryptString(normalized), { mode: 0o600 })
    return { ok: true }
  })

  ipcMain.handle('app:confirmCloudRestoreBatch', async (event, count: number) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win || !Number.isInteger(count) || count < 1) return { ok: false, error: 'Nenhuma foto selecionada.' }
    const label = count === 1 ? 'A foto selecionada será enviada' : `As ${count} imagens selecionadas serão enviadas`
    const confirmation = await dialog.showMessageBox(win, {
      type: 'warning',
      buttons: ['Cancelar', 'Enviar e restaurar'],
      defaultId: 0,
      cancelId: 0,
      title: 'Restauração com IA online',
      message: `${label} à API da OpenAI.`,
      detail: 'Isso sai do modo 100% local e pode gerar cobrança na sua conta. O envio só ocorrerá após esta confirmação.'
    })
    if (confirmation.response !== 1) return { ok: false, error: 'Envio cancelado.' }
    cloudConfirmations.set(event.sender.id, count)
    return { ok: true }
  })

  ipcMain.handle('app:saveRestorationCache', async (_event, sourcePath: string, dataUrl: string) => {
    if (typeof sourcePath !== 'string' || typeof dataUrl !== 'string' || !dataUrl.startsWith('data:image/')) return { ok: false, error: 'Prévia inválida.' }
    try {
      await persistRestorationCache(sourcePath, dataUrl)
      return { ok: true }
    } catch (error) {
      return { ok: false, error: String(error) }
    }
  })

  ipcMain.handle('app:loadRestorationCache', async (_event, items: Array<{ id: number; sourcePath: string }>) => {
    if (!Array.isArray(items)) return {}
    const entries: Record<number, string> = {}
    for (const item of items) {
      if (!Number.isInteger(item?.id) || typeof item.sourcePath !== 'string') continue
      try {
        const source = await stat(item.sourcePath)
        const cached = JSON.parse(await readFile(restorationCachePath(item.sourcePath), 'utf8')) as { sourceSize?: number; sourceMtimeMs?: number; dataUrl?: string }
        if (cached.sourceSize === source.size && cached.sourceMtimeMs === source.mtimeMs && cached.dataUrl?.startsWith('data:image/')) entries[item.id] = cached.dataUrl
      } catch { /* cache ausente, incompleto ou desatualizado */ }
    }
    return entries
  })

  ipcMain.handle('app:cloudRestorePhoto', async (event, sourcePath: string, prompt: string) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    const key = await readOpenAiKey()
    if (!key) return { ok: false, error: 'Configure uma chave da OpenAI antes de usar a IA online.' }
    if (!win || typeof sourcePath !== 'string' || typeof prompt !== 'string') return { ok: false, error: 'Entrada inválida.' }
    const allowance = cloudConfirmations.get(event.sender.id) ?? 0
    if (allowance < 1) return { ok: false, error: 'Confirme o envio do lote antes de iniciar.' }
    if (allowance === 1) cloudConfirmations.delete(event.sender.id)
    else cloudConfirmations.set(event.sender.id, allowance - 1)
    const startedAt = Date.now()
    const usagePath = join(app.getPath('userData'), 'openai-usage.jsonl')
    const writeUsage = async (record: Record<string, unknown>) => {
      try { await appendFile(usagePath, `${JSON.stringify(record)}\n`, 'utf8') } catch { /* relatório nunca bloqueia a restauração */ }
    }
    const baseRecord = {
      timestamp: new Date().toISOString(),
      operation: 'image_edit',
      model: 'gpt-image-2',
      quality: 'high',
      size: 'auto',
      inputFile: sourcePath.split(/[\\/]/).pop() || 'photo',
      promptCharacters: prompt.length
    }
    try {
      const bytes = await readFile(sourcePath)
      const form = new FormData()
      form.append('model', 'gpt-image-2')
      form.append('image', new Blob([bytes], { type: 'image/jpeg' }), sourcePath.split(/[\\/]/).pop() || 'photo.jpg')
      form.append('prompt', prompt)
      form.append('quality', 'high')
      form.append('size', 'auto')
      const response = await fetch('https://api.openai.com/v1/images/edits', {
        method: 'POST',
        headers: { Authorization: `Bearer ${key}` },
        body: form
      })
      const payload = await response.json() as { data?: Array<{ b64_json?: string }>; error?: { message?: string }; usage?: Record<string, unknown> }
      const encoded = payload.data?.[0]?.b64_json
      const record = { ...baseRecord, completedAt: new Date().toISOString(), elapsedMs: Date.now() - startedAt, httpStatus: response.status, status: response.ok && encoded ? 'success' : 'error', requestId: response.headers.get('x-request-id'), inputBytes: bytes.byteLength, outputBytes: encoded ? Buffer.byteLength(encoded, 'base64') : 0, usage: payload.usage ?? null, usageNote: payload.usage ? 'usage informado pela API' : 'tokens não informados na resposta do endpoint de imagem' }
      await writeUsage(record)
      if (!response.ok || !encoded) return { ok: false, error: payload.error?.message ?? `API retornou HTTP ${response.status}.` }
      const dataUrl = `data:image/png;base64,${encoded}`
      try { await persistRestorationCache(sourcePath, dataUrl) } catch (cacheError) { console.error('Falha ao persistir cache de restauração:', cacheError) }
      return { ok: true, dataUrl }
    } catch (error) {
      await writeUsage({ ...baseRecord, completedAt: new Date().toISOString(), elapsedMs: Date.now() - startedAt, status: 'network_error', usage: null, usageNote: 'tokens não informados porque a chamada não completou', error: String(error) })
      return { ok: false, error: `Falha na restauração online: ${String(error)}` }
    }
  })

  ipcMain.handle('app:getOpenAiUsageReport', async () => {
    try {
      const lines = (await readFile(join(app.getPath('userData'), 'openai-usage.jsonl'), 'utf8')).trim().split('\n').filter(Boolean)
      return lines.slice(-200).map((line) => JSON.parse(line))
    } catch { return [] }
  })

  ipcMain.handle('app:exportOpenAiUsageReport', async (event) => {
    const win = BrowserWindow.fromWebContents(event.sender)
    if (!win) return { ok: false, error: 'Janela indisponível.' }
    const usagePath = join(app.getPath('userData'), 'openai-usage.jsonl')
    const result = await dialog.showSaveDialog(win, { title: 'Exportar relatório de uso OpenAI', defaultPath: 'openshoot-openai-usage.jsonl', filters: [{ name: 'Relatório JSONL', extensions: ['jsonl'] }] })
    if (result.canceled || !result.filePath) return { ok: false, error: 'Exportação cancelada.' }
    try { await writeFile(result.filePath, await readFile(usagePath, 'utf8'), 'utf8'); return { ok: true, path: result.filePath } }
    catch (error) { return { ok: false, error: String(error) } }
  })

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
    async (event, dir: string, includeSubdirs: boolean, types: string) => {
      try {
        // Callbacks não atravessam ipcRenderer.invoke (structured clone).
        // Progresso vai por evento no canal dedicado.
        const onProgress = (p: any) => event.sender.send('core:scanProgress', p)
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

  // --- Fase 7: upscale / enhance (ref. Upscayl) ---
  ipcMain.handle('core:upscaleAvailable', (_e, model?: string) => {
    try {
      return getCore().upscaleAvailable(model)
    } catch {
      return false
    }
  })
  ipcMain.handle('core:upscalePhoto', async (_e, id: number, model?: string, scale?: number, maxDim?: number) => {
    try {
      return await getCore().upscalePhoto(id, model, scale, maxDim ?? 512)
    } catch (e) {
      console.error('[upscale] falha upscalePhoto:', e)
      return null
    }
  })
  ipcMain.handle(
    'core:exportUpscaled',
    async (event, ids: number[], destDir: string, model?: string, scale?: number, format?: string, quality?: number) => {
      try {
        // Callback de progresso não cruza invoke (structured clone) — vai por evento.
        const onProgress = (p: any) => event.sender.send('core:upscaleProgress', p)
        return await (getCore() as any).exportUpscaled(
          ids,
          destDir,
          model,
          scale,
          format ?? 'jpeg',
          quality ?? 90,
          onProgress
        )
      } catch (e) {
        return { ok: false, error: String(e) }
      }
    }
  )

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
