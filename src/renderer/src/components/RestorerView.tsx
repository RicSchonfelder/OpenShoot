import { useEffect, useMemo, useRef, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'

type Operation = 'sharpen' | 'denoise' | 'exposure' | 'color' | 'horizon'
type ProcessStatus = 'idle' | 'processing' | 'done' | 'error'
type ComparisonMode = 'side-by-side' | 'before' | 'after' | 'slider'

const OPERATION_LABELS: Record<Operation, string> = {
  sharpen: 'Recuperar nitidez',
  denoise: 'Reduzir ruído',
  exposure: 'Recuperar exposição',
  color: 'Corrigir cores',
  horizon: 'Alinhar horizonte'
}

const DEFAULT_OPERATIONS: Record<Operation, boolean> = {
  sharpen: true,
  denoise: true,
  exposure: true,
  color: true,
  horizon: false
}

interface RestorerViewProps {
  onBack: () => void
  photoIds?: number[]
}

export default function RestorerView({ onBack, photoIds }: RestorerViewProps) {
  const [photos, setPhotos] = useState<PhotoMeta[]>([])
  const [photoId, setPhotoId] = useState<number | null>(null)
  const [selectedIds, setSelectedIds] = useState<Set<number>>(new Set())
  const [photoThumbs, setPhotoThumbs] = useState<Record<number, string>>({})
  const [operations, setOperations] = useState(DEFAULT_OPERATIONS)
  const [strength, setStrength] = useState<'conservative' | 'normal' | 'strong'>('conservative')
  const [previews, setPreviews] = useState<Record<number, string>>({})
  const [statuses, setStatuses] = useState<Record<number, ProcessStatus>>({})
  const [original, setOriginal] = useState<string | null>(null)
  const [originalLoading, setOriginalLoading] = useState(false)
  const [busy, setBusy] = useState(false)
  const [progress, setProgress] = useState('')
  const [zoom, setZoom] = useState(1)
  const [comparisonMode, setComparisonMode] = useState<ComparisonMode>('side-by-side')
  const [splitPosition, setSplitPosition] = useState(50)
  const [outputDir, setOutputDir] = useState<string | null>(null)
  const [cloudModel, setCloudModel] = useState('gpt-image-2')
  const [message, setMessage] = useState<string | null>(null)
  const [openAiKey, setOpenAiKey] = useState('')
  const [keyConfigured, setKeyConfigured] = useState(false)
  const [editingKey, setEditingKey] = useState(false)
  const [panningViewport, setPanningViewport] = useState<'before' | 'after' | null>(null)
  const beforeViewportRef = useRef<HTMLDivElement | null>(null)
  const afterViewportRef = useRef<HTMLDivElement | null>(null)
  const panStateRef = useRef<{ source: 'before' | 'after'; x: number; y: number; scrollLeft: number; scrollTop: number } | null>(null)
  const splitDraggingRef = useRef(false)

  const photo = useMemo(() => photos.find((item) => item.id === photoId) ?? null, [photos, photoId])

  useEffect(() => {
    window.openshoot.listPhotos('', 'all', 0, 1000).then((result) => {
      const scopedPhotos = photoIds ? result.photos.filter((item) => photoIds.includes(item.id)) : result.photos
      setPhotos(scopedPhotos)
      if (scopedPhotos[0]) {
        setPhotoId(scopedPhotos[0].id)
        setSelectedIds(new Set([scopedPhotos[0].id]))
      }
    }).catch(() => setMessage('Não foi possível carregar o catálogo.'))
  }, [photoIds])

  useEffect(() => {
    if (!photos.length) return
    window.openshoot.loadRestorationCache(photos.map((item) => ({ id: item.id, sourcePath: item.path }))).then((cached) => {
      const ids = Object.keys(cached).map(Number)
      if (!ids.length) return
      setPreviews((current) => ({ ...current, ...cached }))
      setStatuses((current) => ({ ...current, ...Object.fromEntries(ids.map((id) => [id, 'done' as ProcessStatus])) }))
    }).catch(() => {})
  }, [photos])

  useEffect(() => {
    let cancelled = false
    Promise.all(photos.map(async (item) => {
      try { return [item.id, await window.openshoot.thumbForPhoto(item.id, 240)] as const } catch { return null }
    })).then((entries) => {
      if (!cancelled) setPhotoThumbs(Object.fromEntries(entries.filter((entry): entry is readonly [number, string] => Boolean(entry))))
    })
    return () => { cancelled = true }
  }, [photos])

  useEffect(() => {
    window.openshoot.hasOpenAiKey().then(setKeyConfigured).catch(() => setKeyConfigured(false))
  }, [])

  useEffect(() => {
    setOriginal(null)
    setOriginalLoading(true)
    setZoom(1)
    if (!photo) return
    window.openshoot.thumbForPhoto(photo.id, 1200).then(setOriginal).catch(() => {}).finally(() => setOriginalLoading(false))
  }, [photo])

  const preview = photoId != null ? (previews[photoId] ?? null) : null
  const selectedPhotos = useMemo(() => photos.filter((item) => selectedIds.has(item.id)), [photos, selectedIds])

  const selectPhoto = (id: number) => setPhotoId(id)

  const togglePhoto = (id: number) => {
    setSelectedIds((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      if (!next.size) next.add(id)
      return next
    })
    setPhotoId(id)
  }

  const buildRecipe = (): string => {
    const multiplier = strength === 'conservative' ? 0.55 : strength === 'strong' ? 1.35 : 1
    const recipe: Record<string, number> = {}
    if (operations.sharpen) recipe.sharpen = Math.round(38 * multiplier)
    if (operations.denoise) recipe.denoise = Math.round(28 * multiplier)
    if (operations.exposure) recipe.exposure = 0.15 * multiplier
    if (operations.color) {
      recipe.temperature = 6500
      recipe.tint = 0
    }
    return JSON.stringify(recipe)
  }

  const runLocalRestore = async () => {
    if (!selectedPhotos.length || !Object.values(operations).some(Boolean)) return
    setBusy(true)
    setMessage(null)
    setProgress(`Processando 0/${selectedPhotos.length}`)
    let completed = 0
    let failures = 0
    for (const item of selectedPhotos) {
      setStatuses((current) => ({ ...current, [item.id]: 'processing' }))
      try {
        const result = await window.openshoot.previewEdit(item.id, buildRecipe(), 1600)
        if (!result) throw new Error('O core não retornou uma prévia.')
        setPreviews((current) => ({ ...current, [item.id]: result }))
        await window.openshoot.saveRestorationCache(item.path, result)
        setStatuses((current) => ({ ...current, [item.id]: 'done' }))
      } catch {
        failures += 1
        setStatuses((current) => ({ ...current, [item.id]: 'error' }))
      }
      completed += 1
      setProgress(`Processando ${completed}/${selectedPhotos.length}`)
    }
    setBusy(false)
    setProgress('')
    setMessage(failures ? `${completed - failures} foto(s) restaurada(s); ${failures} falha(s).` : `${completed} foto(s) restaurada(s) com as ferramentas locais.`)
  }

  const save = async () => {
    if (!preview || !photo) return
    setBusy(true)
    const result = await window.openshoot.saveRestoredPreview(preview, `${photo.fileName.replace(/\.[^.]+$/, '')}-restaurada`)
    setMessage(result.ok ? `Cópia salva em ${result.path}` : (result.error ?? 'Salvamento cancelado.'))
    setBusy(false)
  }

  const saveBatch = async () => {
    const items = selectedPhotos.flatMap((item) => previews[item.id] ? [{ dataUrl: previews[item.id], defaultName: `${item.fileName.replace(/\.[^.]+$/, '')}-restaurada` }] : [])
    if (!items.length) return
    setBusy(true)
    let dir = outputDir
    if (!dir) {
      const picked = await window.openshoot.pickRestorationFolder()
      if (!picked.ok || !picked.path) {
        setBusy(false)
        setMessage('Salvamento cancelado.')
        return
      }
      dir = picked.path
      setOutputDir(dir)
    }
    const result = await window.openshoot.saveRestoredPreviews(items, dir)
    setMessage(result.ok ? `${result.saved ?? items.length} cópia(s) salva(s) em ${result.path}` : (result.error ?? 'Salvamento cancelado.'))
    setBusy(false)
  }

  const chooseOutputDir = async () => {
    if (busy) return
    const result = await window.openshoot.pickRestorationFolder()
    if (result.ok && result.path) {
      setOutputDir(result.path)
      setMessage(`Destino definido: ${result.path}`)
    }
  }

  const syncViewport = (source: 'before' | 'after') => {
    const from = source === 'before' ? beforeViewportRef.current : afterViewportRef.current
    const to = source === 'before' ? afterViewportRef.current : beforeViewportRef.current
    if (from && to) {
      to.scrollLeft = from.scrollLeft
      to.scrollTop = from.scrollTop
    }
  }

  const startPan = (source: 'before' | 'after', event: React.PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || zoom <= 1) return
    event.preventDefault()
    event.currentTarget.setPointerCapture(event.pointerId)
    panStateRef.current = { source, x: event.clientX, y: event.clientY, scrollLeft: event.currentTarget.scrollLeft, scrollTop: event.currentTarget.scrollTop }
    setPanningViewport(source)
  }

  const movePan = (event: React.PointerEvent<HTMLDivElement>) => {
    const pan = panStateRef.current
    if (!pan || !event.currentTarget.hasPointerCapture(event.pointerId)) return
    event.currentTarget.scrollLeft = pan.scrollLeft - (event.clientX - pan.x)
    event.currentTarget.scrollTop = pan.scrollTop - (event.clientY - pan.y)
    syncViewport(pan.source)
  }

  const stopPan = (event: React.PointerEvent<HTMLDivElement>) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
    panStateRef.current = null
    setPanningViewport(null)
  }

  const setSplitFromPointer = (event: React.PointerEvent<HTMLDivElement>) => {
    const rect = event.currentTarget.getBoundingClientRect()
    setSplitPosition(Math.min(100, Math.max(0, ((event.clientX - rect.left) / rect.width) * 100)))
  }

  const startSplitDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    splitDraggingRef.current = true
    event.currentTarget.setPointerCapture(event.pointerId)
    setSplitFromPointer(event)
  }

  const moveSplitDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    if (!splitDraggingRef.current || !event.currentTarget.hasPointerCapture(event.pointerId)) return
    event.preventDefault()
    setSplitFromPointer(event)
  }

  const stopSplitDrag = (event: React.PointerEvent<HTMLDivElement>) => {
    splitDraggingRef.current = false
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId)
  }

  const handleWheel = (source: 'before' | 'after', event: React.WheelEvent<HTMLDivElement>) => {
    const viewport = event.currentTarget
    const oldZoom = zoom
    const nextZoom = Math.min(3, Math.max(0.5, oldZoom * Math.exp(-event.deltaY * 0.0015)))
    if (nextZoom === oldZoom) return
    event.preventDefault()
    const rect = viewport.getBoundingClientRect()
    const localX = event.clientX - rect.left
    const localY = event.clientY - rect.top
    const imageX = (viewport.scrollLeft + localX) / oldZoom
    const imageY = (viewport.scrollTop + localY) / oldZoom
    setZoom(nextZoom)
    requestAnimationFrame(() => {
      viewport.scrollLeft = Math.max(0, imageX * nextZoom - localX)
      viewport.scrollTop = Math.max(0, imageY * nextZoom - localY)
      syncViewport(source)
    })
  }

  const cloudPrompt = `Restore and enhance this photograph while preserving the original image, people, environment, composition, and moment exactly as captured.

This is a PHOTO RESTORATION task, not an image recreation.

Prioritize natural photographic realism and identity preservation.

Carefully improve:
- mild motion blur and micro-camera shake
- slightly soft or missed focus
- facial clarity without reconstructing faces
- natural detail in hair, beard, eyebrows, clothing and objects
- fine texture and edge definition
- noise caused by indoor low-light photography
- excessive smartphone noise reduction and smearing
- local contrast and tonal separation
- highlight and shadow recovery
- subtle white balance correction
- JPEG/compression artifacts if present

Faces are the highest priority.

Preserve each person's exact facial geometry, proportions, expression, eyes, nose, mouth, beard, hairline, skin tone and distinctive characteristics.

Do NOT beautify people.
Do NOT change facial features.
Do NOT regenerate faces.
Do NOT alter expressions.
Do NOT make skin artificially smooth.
Do NOT add eyelashes, hair, beard strands, teeth or facial details that cannot reasonably be recovered from the source.

Recover apparent sharpness conservatively. If information is genuinely missing because of motion blur or defocus, preserve a small amount of natural softness rather than hallucinating detail.

Maintain realistic skin texture with pores and natural imperfections, but never manufacture exaggerated skin detail.

Preserve all text, logos, clothing, badges, objects and background elements exactly as they appear in the original photograph.

Do not replace, redesign or reinterpret anything.

Maintain the original:
- framing
- perspective
- body positions
- hands and fingers
- clothing
- Electrolux product box
- event background
- flowers
- lighting direction
- depth of field

Apply realistic photographic sharpening rather than AI-generated hyper-detail.

Reduce noise while retaining genuine texture.

Avoid:
AI face reconstruction, plastic skin, oversharpening, halos, HDR look, fake texture, artificial eyes, invented hair strands, excessive clarity, cinematic relighting, background replacement, object modification, text alteration or generative reconstruction.

The final result should look like the SAME photograph captured with a better exposure, steadier camera and cleaner photographic processing — not like an AI-generated version of the scene.

Photorealistic, natural, documentary photography, identity-preserving restoration.`

  const configureKey = async () => {
    const result = await window.openshoot.saveOpenAiKey(openAiKey)
    if (result.ok) {
      setOpenAiKey('')
      setKeyConfigured(true)
      setEditingKey(false)
      setMessage('Chave salva no armazenamento seguro do sistema.')
    } else setMessage(result.error ?? 'Não foi possível salvar a chave.')
  }

  const runCloudRestore = async () => {
    if (!selectedPhotos.length || !keyConfigured) return
    let dir = outputDir
    if (!dir) {
      const picked = await window.openshoot.pickRestorationFolder()
      if (!picked.ok || !picked.path) {
        setMessage('Envio cancelado: escolha uma pasta de destino antes de iniciar.')
        return
      }
      dir = picked.path
      setOutputDir(dir)
    }
    const pendingPhotos = selectedPhotos.filter((item) => !previews[item.id])
    if (!pendingPhotos.length) {
      setMessage('Todas as fotos selecionadas já têm restauração em cache; nenhuma nova chamada foi feita.')
      return
    }
    setBusy(true)
    setMessage(null)
    setProgress(`Processando 0/${pendingPhotos.length}`)
    const confirmation = await window.openshoot.confirmCloudRestoreBatch(pendingPhotos.length)
    if (!confirmation.ok) {
      setBusy(false)
      setMessage(confirmation.error ?? 'Envio cancelado.')
      return
    }
    let completed = 0
    let failures = 0
    const concurrency = Math.min(3, pendingPhotos.length)
    let nextIndex = 0
    const worker = async () => {
      while (nextIndex < pendingPhotos.length) {
        const item = pendingPhotos[nextIndex++]
        setStatuses((current) => ({ ...current, [item.id]: 'processing' }))
        try {
      const horizonPrompt = operations.horizon ? `

Correct the camera alignment and straighten the photograph naturally.

Identify the true horizontal and vertical architectural references in the original scene and apply only the minimum rotation necessary to make the photograph visually level.

Correct minor perspective distortion if necessary, while preserving the original camera perspective and proportions.

After straightening, crop only the minimum amount required at the outer edges.

Do NOT generate, extend, outpaint, reconstruct or invent any content to fill empty areas created by the rotation.

Preserve the original aspect ratio whenever possible.

The final image should look exactly like the same photograph taken with the camera properly leveled at the moment of capture.

Use structural references such as stage edges, table lines, walls, floors and architectural lines. Do not use individual people as references for determining the horizon.` : ''
      const result = await window.openshoot.cloudRestorePhoto(item.path, cloudPrompt + horizonPrompt, cloudModel)
      if (!result.ok || !result.dataUrl) throw new Error(result.error ?? 'A API não retornou uma imagem.')
      setPreviews((current) => ({ ...current, [item.id]: result.dataUrl! }))
      await window.openshoot.saveRestorationCache(item.path, result.dataUrl)
      setStatuses((current) => ({ ...current, [item.id]: 'done' }))
      } catch {
        failures += 1
        setStatuses((current) => ({ ...current, [item.id]: 'error' }))
      }
      completed += 1
      setProgress(`Processando ${completed}/${pendingPhotos.length}`)
      }
    }
    await Promise.all(Array.from({ length: concurrency }, () => worker()))
    setBusy(false)
    setProgress('')
    setMessage(failures ? `${completed - failures} nova(s) foto(s) restaurada(s) online; ${failures} falha(s). Destino: ${dir}` : `${completed} foto(s) restaurada(s) com IA online. Destino: ${dir}`)
  }

  const exportUsageReport = async () => {
    const result = await window.openshoot.exportOpenAiUsageReport()
    setMessage(result.ok ? `Relatório de uso exportado para ${result.path}` : (result.error ?? 'Não foi possível exportar o relatório.'))
  }

  return (
    <div className="app restorer-view">
      <header className="topbar">
        <span className="logo">OpenShoot · Bancada de restauração</span>
        <button className="ghost" onClick={onBack}>← Voltar</button>
      </header>
      <main className="restorer-body">
        <aside className="restorer-controls">
          <h2>Teste de recuperação</h2>
          <p className="restorer-hint">Selecione uma ou mais fotos. O original permanece intocado; os resultados são cópias.</p>
          <button className="ghost restorer-select-all" onClick={() => setSelectedIds(new Set(photos.map((item) => item.id)))} disabled={!photos.length || busy}>Selecionar todas ({photos.length})</button>
          <fieldset>
            <legend>Melhorias</legend>
            {(Object.keys(OPERATION_LABELS) as Operation[]).map((key) => (
              <label className="restorer-check" key={key}>
                <input type="checkbox" checked={operations[key]} onChange={(event) => setOperations({ ...operations, [key]: event.target.checked })} />
                {OPERATION_LABELS[key]}
              </label>
            ))}
          </fieldset>
          <label className="restorer-field">Intensidade
            <select value={strength} onChange={(event) => setStrength(event.target.value as typeof strength)}>
              <option value="conservative">Conservadora (recomendada)</option>
              <option value="normal">Normal</option>
              <option value="strong">Forte</option>
            </select>
          </label>
          <button className="primary" onClick={runLocalRestore} disabled={!selectedPhotos.length || busy || !Object.values(operations).some(Boolean)}>
            {busy ? (progress || 'Processando…') : 'Restaurar com ferramentas locais'}
          </button>
          <div className="restorer-online">
            <h3>IA online experimental</h3>
            <p>Envia a foto à OpenAI e pode gerar cobrança. Só funciona após confirmação.</p>
            <label className="restorer-field">Modelo para este lote
              <select value={cloudModel} onChange={(event) => setCloudModel(event.target.value)} disabled={busy}>
                <option value="gpt-image-2">gpt-image-2 · atual</option>
                <option value="gpt-image-1">gpt-image-1 · qualidade</option>
                <option value="gpt-image-1-mini">gpt-image-1-mini · econômico</option>
              </select>
            </label>
            {keyConfigured && !editingKey && <>
              <p className="restorer-key-status">Chave configurada com segurança. O valor nunca é exibido.</p>
              <button className="ghost" onClick={() => { setOpenAiKey(''); setEditingKey(true); setMessage(null) }} disabled={busy}>Trocar chave</button>
            </>}
            {(!keyConfigured || editingKey) && <>
              <input type="password" placeholder="Chave OpenAI (sk-…)" value={openAiKey} onChange={(event) => setOpenAiKey(event.target.value)} />
              <button className="ghost" onClick={configureKey} disabled={!openAiKey.trim()}>Salvar {keyConfigured ? 'nova ' : ''}chave com segurança</button>
              {keyConfigured && <button className="ghost" onClick={() => { setOpenAiKey(''); setEditingKey(false); setMessage(null) }} disabled={busy}>Cancelar troca</button>}
            </>}
            <button className="online-button" onClick={runCloudRestore} disabled={!selectedPhotos.length || !keyConfigured || busy}>
              Restaurar com IA online
            </button>
            <button className="ghost" onClick={chooseOutputDir} disabled={busy}>{outputDir ? 'Trocar pasta de destino' : 'Escolher pasta de destino'}</button>
            {outputDir && <p className="restorer-destination">Destino: {outputDir}</p>}
            <button className="ghost" onClick={exportUsageReport}>Exportar relatório de uso</button>
          </div>
          <button className="ghost" onClick={save} disabled={!preview || busy}>Salvar cópia JPEG</button>
          <button className="ghost" onClick={saveBatch} disabled={!selectedPhotos.some((item) => Boolean(previews[item.id])) || busy}>Salvar lote em uma pasta</button>
          {message && <p className="restorer-message">{message}</p>}
        </aside>
        <section className="restorer-stage">
          <div className="restorer-toolbar">
            <strong>{selectedPhotos.length} selecionada(s)</strong>
            <div className="restorer-view-actions">
              <button className={comparisonMode === 'side-by-side' ? 'active' : ''} onClick={() => setComparisonMode('side-by-side')}>Lado a lado</button>
              <button className={comparisonMode === 'before' ? 'active' : ''} onClick={() => setComparisonMode('before')}>Antes</button>
              <button className={comparisonMode === 'after' ? 'active' : ''} onClick={() => setComparisonMode('after')}>Depois</button>
              <button className={comparisonMode === 'slider' ? 'active' : ''} onClick={() => setComparisonMode('slider')}>Comparar</button>
              <button onClick={() => setZoom(Math.max(0.5, zoom - 0.25))}>−</button>
              <span>{Math.round(zoom * 100)}%</span>
              <button onClick={() => setZoom(Math.min(3, zoom + 0.25))}>+</button>
              <button onClick={() => setZoom(1)}>1:1</button>
            </div>
          </div>
          <div className={`restorer-images comparison-${comparisonMode}`}>
            {comparisonMode === 'slider' ? <div className="restorer-slider-comparison">
              <span>Comparação deslizante</span>
              <div className="restorer-slider-viewport" onPointerDown={startSplitDrag} onPointerMove={moveSplitDrag} onPointerUp={stopSplitDrag} onPointerCancel={stopSplitDrag}>
                {original && <img className="restorer-slider-before" src={original} alt="Original" draggable={false} style={{ transform: `scale(${zoom})` }} />}
                {preview && <img className="restorer-slider-after" src={preview} alt="Restaurada" draggable={false} style={{ clipPath: `inset(0 ${100 - splitPosition}% 0 0)`, transform: `scale(${zoom})` }} />}
                <div className="restorer-slider-handle" style={{ left: `${splitPosition}%` }}><span>↔</span></div>
              </div>
            </div> : <>
            {comparisonMode !== 'after' && <div><span>Antes · original</span>{originalLoading ? <div className="restorer-empty"><span className="restorer-spinner" /> Carregando original…</div> : original ? <div ref={beforeViewportRef} className={`restorer-image-viewport ${panningViewport === 'before' ? 'is-panning' : ''}`} onWheel={(event) => handleWheel('before', event)} onScroll={() => syncViewport('before')} onPointerDown={(event) => startPan('before', event)} onPointerMove={movePan} onPointerUp={stopPan} onPointerCancel={stopPan}><img src={original} alt="Original" style={{ width: `${zoom * 100}%`, height: `${zoom * 100}%` }} /></div> : <div className="restorer-empty">Selecione uma foto</div>}</div>}
            {comparisonMode !== 'before' && <div><span>Depois · restaurada</span>{preview ? <div ref={afterViewportRef} className={`restorer-image-viewport ${panningViewport === 'after' ? 'is-panning' : ''}`} onWheel={(event) => handleWheel('after', event)} onScroll={() => syncViewport('after')} onPointerDown={(event) => startPan('after', event)} onPointerMove={movePan} onPointerUp={stopPan} onPointerCancel={stopPan}><img src={preview} alt="Prévia restaurada" style={{ width: `${zoom * 100}%`, height: `${zoom * 100}%` }} /></div> : <div className="restorer-empty">Restaure para comparar</div>}</div>}
            </>}
          </div>
          <div className="restorer-filmstrip">
            {photos.map((item) => <button key={item.id} className={`restorer-thumb ${item.id === photoId ? 'current' : ''} ${selectedIds.has(item.id) ? 'selected' : ''}`} onClick={() => selectPhoto(item.id)} title={item.fileName}>
              {photoThumbs[item.id] ? <img src={photoThumbs[item.id]} alt="" /> : <span className="restorer-thumb-placeholder">…</span>}
              <span className="restorer-thumb-check" onClick={(event) => { event.stopPropagation(); togglePhoto(item.id) }}>{selectedIds.has(item.id) ? '✓' : '○'}</span>
              <small>{item.fileName}</small>
              {statuses[item.id] === 'processing' && <i className="restorer-thumb-status">Processando…</i>}
              {statuses[item.id] === 'done' && <i className="restorer-thumb-status done">Pronto</i>}
              {statuses[item.id] === 'error' && <i className="restorer-thumb-status error">Falhou</i>}
            </button>)}
          </div>
          <p className="restorer-footnote">A prévia usa o pipeline local do OpenShoot e pode ter dimensão reduzida para o teste. Nenhuma foto é enviada pela rede.</p>
        </section>
      </main>
    </div>
  )
}
