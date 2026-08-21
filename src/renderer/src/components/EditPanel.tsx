import { useCallback, useEffect, useRef, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'
import { useT } from '../i18n/I18nContext'

export interface EditValues {
  exposure: number | null
  temperature: number | null
  tint: number | null
  contrast: number | null
  saturation: number | null
  shadows: number | null
  highlights: number | null
  brightness: number | null
  toneHighlights: number | null
  toneLights: number | null
  toneDarks: number | null
  toneShadows: number | null
  hsl: number[] | null
  sharpen: number | null
  denoise: number | null
}

const EMPTY: EditValues = {
  exposure: null,
  temperature: null,
  tint: null,
  contrast: null,
  saturation: null,
  shadows: null,
  highlights: null,
  brightness: null,
  toneHighlights: null,
  toneLights: null,
  toneDarks: null,
  toneShadows: null,
  hsl: null,
  sharpen: null,
  denoise: null
}

const HSL_COLORS = [
  'Red',
  'Orange',
  'Yellow',
  'Green',
  'Aqua',
  'Blue',
  'Purple',
  'Magenta'
] as const

interface SliderDef {
  key: 'exposure' | 'temperature' | 'tint' | 'contrast' | 'saturation' | 'shadows' | 'highlights' | 'brightness'
  labelKey: string
  min: number
  max: number
  step: number
  unit: string
}

const SLIDERS: SliderDef[] = [
  { key: 'exposure', labelKey: 'edit.exposicao', min: -3, max: 3, step: 0.05, unit: 'EV' },
  { key: 'temperature', labelKey: 'edit.temperatura', min: 2000, max: 12000, step: 50, unit: 'K' },
  { key: 'tint', labelKey: 'edit.tint', min: -100, max: 100, step: 1, unit: '' },
  { key: 'contrast', labelKey: 'edit.contraste', min: -100, max: 100, step: 1, unit: '' },
  { key: 'saturation', labelKey: 'edit.saturacao', min: -100, max: 100, step: 1, unit: '' },
  { key: 'shadows', labelKey: 'edit.sombras', min: -100, max: 100, step: 1, unit: '' },
  { key: 'highlights', labelKey: 'edit.realces', min: -100, max: 100, step: 1, unit: '' },
  { key: 'brightness', labelKey: 'edit.brilho', min: -100, max: 100, step: 1, unit: '' }
]

const TONE_SLIDERS: Array<{ key: 'toneHighlights' | 'toneLights' | 'toneDarks' | 'toneShadows'; labelKey: string }> = [
  { key: 'toneHighlights', labelKey: 'edit.curvaDestaques' },
  { key: 'toneLights', labelKey: 'edit.curvaLuzes' },
  { key: 'toneDarks', labelKey: 'edit.curvaEscuros' },
  { key: 'toneShadows', labelKey: 'edit.curvaSombras' }
]

function toJson(values: EditValues): string {
  const o: Record<string, number | number[]> = {}
  for (const s of SLIDERS) {
    const v = values[s.key]
    if (v == null) continue
    const neutral = s.key === 'temperature' ? 6500 : 0
    if (v !== neutral) o[s.key] = v
  }
  const curve = [
    values.toneHighlights ?? 0,
    values.toneLights ?? 0,
    values.toneDarks ?? 0,
    values.toneShadows ?? 0
  ]
  if (curve.some((v) => v !== 0)) o.tone_curve = curve
  if (values.hsl && values.hsl.some((v) => v !== 0)) o.hsl = values.hsl
  if (values.sharpen != null && values.sharpen !== 0) o.sharpen = values.sharpen
  if (values.denoise != null && values.denoise !== 0) o.denoise = values.denoise
  return JSON.stringify(o)
}

interface EditPanelProps {
  photo: PhotoMeta | null
  onApplyAll: (json: string) => void
}

interface PresetItem {
  name: string
  recipe: string
}

export default function EditPanel({ photo, onApplyAll }: EditPanelProps) {
  const { t } = useT()
  const [values, setValues] = useState<EditValues>(EMPTY)
  const [skinIntensity, setSkinIntensity] = useState(0)
  const [preview, setPreview] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [presets, setPresets] = useState<PresetItem[]>([])
  const [presetName, setPresetName] = useState('')
  const [hslColor, setHslColor] = useState(0)
  const [faceRegions, setFaceRegions] = useState<Record<string, number>>({})
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Carrega a lista de presets salvos.
  const loadPresets = useCallback(() => {
    window.openshoot
      .listPresets()
      .then((p) => setPresets(p))
      .catch(() => {})
  }, [])

  useEffect(() => {
    loadPresets()
  }, [loadPresets])

  const applyRecipe = useCallback(
    (recipe: string) => {
      try {
        const p = JSON.parse(recipe)
        const c = p.tone_curve as number[] | undefined
        const next: EditValues = {
          exposure: p.exposure ?? null,
          temperature: p.temperature ?? null,
          tint: p.tint ?? null,
          contrast: p.contrast ?? null,
          saturation: p.saturation ?? null,
          shadows: p.shadows ?? null,
          highlights: p.highlights ?? null,
          brightness: p.brightness ?? null,
          toneHighlights: c?.[0] ?? null,
          toneLights: c?.[1] ?? null,
          toneDarks: c?.[2] ?? null,
          toneShadows: c?.[3] ?? null,
          hsl: Array.isArray(p.hsl) ? (p.hsl as number[]) : null,
          sharpen: p.sharpen ?? null,
          denoise: p.denoise ?? null
        }
        setValues(next)
        if (photo) updatePreview(next)
      } catch {
        /* ignore */
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [photo?.id]
  )

  const savePresetNow = useCallback(() => {
    const name = presetName.trim()
    if (!name) return
    window.openshoot
      .savePreset(name, toJson(values))
      .then(() => {
        setPresetName('')
        loadPresets()
      })
      .catch(() => {})
  }, [presetName, values, loadPresets])

  const removePreset = useCallback(
    (name: string) => {
      window.openshoot
        .deletePreset(name)
        .then(() => loadPresets())
        .catch(() => {})
    },
    [loadPresets]
  )

  // Ao trocar de foto, carrega a receita salva (se houver).
  useEffect(() => {
    setValues(EMPTY)
    setPreview(null)
    if (!photo) return
    window.openshoot.getPhotoEdit(photo.id).then((json) => {
      if (!json) return
      try {
        const p = JSON.parse(json)
        const c = p.tone_curve as number[] | undefined
        setValues({
          exposure: p.exposure ?? null,
          temperature: p.temperature ?? null,
          tint: p.tint ?? null,
          contrast: p.contrast ?? null,
          saturation: p.saturation ?? null,
          shadows: p.shadows ?? null,
          highlights: p.highlights ?? null,
          brightness: p.brightness ?? null,
          toneHighlights: c?.[0] ?? null,
          toneLights: c?.[1] ?? null,
          toneDarks: c?.[2] ?? null,
          toneShadows: c?.[3] ?? null,
          hsl: Array.isArray(p.hsl) ? (p.hsl as number[]) : null,
          sharpen: p.sharpen ?? null,
          denoise: p.denoise ?? null
        })
      } catch {
        /* ignore */
      }
    })
  }, [photo?.id])

  const updatePreview = useCallback(
    (vals: EditValues) => {
      if (!photo) return
      const json = toJson(vals)
      setBusy(true)
      window.openshoot
        .previewEdit(photo.id, json, 400)
        .then((t) => t && setPreview(t))
        .catch(() => {})
        .finally(() => setBusy(false))
    },
    [photo?.id]
  )

  const onSlider = (key: SliderDef['key'], val: number, neutral: number) => {
    const next = { ...values, [key]: val === neutral ? null : val }
    setValues(next)
    if (debounce.current) clearTimeout(debounce.current)
    debounce.current = setTimeout(() => updatePreview(next), 200)
  }

  const onToneSlider = (key: keyof EditValues, val: number) => {
    const next = { ...values, [key]: val === 0 ? null : val }
    setValues(next)
    if (debounce.current) clearTimeout(debounce.current)
    debounce.current = setTimeout(() => updatePreview(next), 200)
  }

  const onHslSlider = (channel: number, val: number) => {
    const hsl = values.hsl ? [...values.hsl] : new Array(24).fill(0)
    hsl[hslColor * 3 + channel] = val
    const allZero = hsl.every((v) => v === 0)
    const next = { ...values, hsl: allZero ? null : hsl }
    setValues(next)
    if (debounce.current) clearTimeout(debounce.current)
    debounce.current = setTimeout(() => updatePreview(next), 200)
  }

  const onDetailSlider = (key: 'sharpen' | 'denoise', val: number) => {
    const next = { ...values, [key]: val === 0 ? null : val }
    setValues(next)
    if (debounce.current) clearTimeout(debounce.current)
    debounce.current = setTimeout(() => updatePreview(next), 200)
  }

  const applyCurrent = () => {
    if (!photo) return
    const json = toJson(values)
    setBusy(true)
    window.openshoot
      .applyEditOne(photo.id, json, 400)
      .then((t) => t && setPreview(t))
      .finally(() => setBusy(false))
    onApplyAll(json)
  }

  const removeDistraction = () => {
    if (!photo) return
    setBusy(true)
    // MVP: remove uma região central (bbox normalizada). Seleção por arrasto
    // virá numa próxima iteração.
    window.openshoot
      .inpaintPhoto(photo.id, [0.35, 0.35, 0.65, 0.65], 400)
      .then((t) => t && setPreview(t))
      .catch(() => {})
      .finally(() => setBusy(false))
  }

  const autoLevel = () => {
    if (!photo) return
    setBusy(true)
    window.openshoot
      .autoLevelPhoto(photo.id, 400)
      .then((res) => {
        if ('preview' in res) setPreview(res.preview)
      })
      .catch(() => {})
      .finally(() => setBusy(false))
  }

  const aiCrop = () => {
    if (!photo) return
    setBusy(true)
    window.openshoot
      .aiCropPhoto(photo.id, 400)
      .then((t) => t && setPreview(t))
      .catch(() => {})
      .finally(() => setBusy(false))
  }

  if (!photo) {
    return (
      <aside className="edit-panel">
        <h3>{t('edit.titulo')}</h3>
        <p className="edit-hint">{t('edit.hint')}</p>
      </aside>
    )
  }

  return (
    <aside className="edit-panel">
      <h3>{t('edit.tituloFoto', { name: photo.fileName })}</h3>
      <div className="edit-preview">
        {preview ? (
          <img src={preview} alt="preview editado" />
        ) : (
          <div className="edit-preview-empty">
            {busy ? t('edit.processando') : t('edit.ajusteControles')}
          </div>
        )}
      </div>
      <div className="edit-sliders">
        {SLIDERS.map((s) => {
          const neutral = s.key === 'temperature' ? 6500 : 0
          const v = values[s.key] ?? neutral
          return (
            <label key={s.key} className="edit-slider">
              <span>
                {t(s.labelKey)}
                <em>
                  {v}
                  {s.unit}
                </em>
              </span>
              <input
                type="range"
                min={s.min}
                max={s.max}
                step={s.step}
                value={v}
                onChange={(e) => onSlider(s.key, Number(e.target.value), neutral)}
              />
            </label>
          )
        })}
      </div>

      <div className="edit-tonecurve">
        <h4>{t('edit.curvaTitulo')}</h4>
        {TONE_SLIDERS.map((s) => {
          const v = values[s.key] ?? 0
          return (
            <label key={s.key} className="edit-slider">
              <span>
                {t(s.labelKey)}
                <em>{v}</em>
              </span>
              <input
                type="range"
                min={-100}
                max={100}
                step={1}
                value={v}
                onChange={(e) => onToneSlider(s.key, Number(e.target.value))}
              />
            </label>
          )
        })}
      </div>

      <div className="edit-hsl">
        <h4>{t('edit.hslTitulo')}</h4>
        <div className="hsl-colors">
          {HSL_COLORS.map((c, i) => (
            <button
              key={c}
              className={`hsl-color hsl-${c.toLowerCase()} ${hslColor === i ? 'active' : ''}`}
              onClick={() => setHslColor(i)}
            >
              {c.slice(0, 3)}
            </button>
          ))}
        </div>
        {(
          [
            ['edit.hslMatiz', 0],
            ['edit.hslSaturacao', 1],
            ['edit.hslLuminancia', 2]
          ] as Array<[string, number]>
        ).map(([labelKey, channel]) => {
          const v = values.hsl?.[hslColor * 3 + channel] ?? 0
          return (
            <label key={labelKey} className="edit-slider">
              <span>
                {t(labelKey)}
                <em>{v}</em>
              </span>
              <input
                type="range"
                min={-100}
                max={100}
                step={1}
                value={v}
                onChange={(e) => onHslSlider(channel, Number(e.target.value))}
              />
            </label>
          )
        })}
      </div>

      <div className="edit-sharp">
        <h4>{t('edit.nitidezTitulo')}</h4>
        {(
          [
            ['edit.nitidez', 'sharpen'],
            ['edit.ruido', 'denoise']
          ] as Array<[string, 'sharpen' | 'denoise']>
        ).map(([labelKey, key]) => {
          const v = values[key] ?? 0
          return (
            <label key={key} className="edit-slider">
              <span>
                {t(labelKey)}
                <em>{v}</em>
              </span>
              <input
                type="range"
                min={-100}
                max={100}
                step={1}
                value={v}
                onChange={(e) => onDetailSlider(key, Number(e.target.value))}
              />
            </label>
          )
        })}
      </div>

      <div className="edit-retouch">
        <h4>{t('edit.retoque')}</h4>
        <label className="edit-slider">
          <span>
            {t('edit.suavizacaoPele')}
            <em>{Math.round(skinIntensity * 100)}%</em>
          </span>
          <input
            type="range"
            min={0}
            max={100}
            step={1}
            value={Math.round(skinIntensity * 100)}
            onChange={(e) => {
              const v = Number(e.target.value) / 100
              setSkinIntensity(v)
              if (debounce.current) clearTimeout(debounce.current)
              debounce.current = setTimeout(() => {
                setBusy(true)
                window.openshoot
                  .retouchSkinPhoto(photo.id, v, 400)
                  .then((t) => t && setPreview(t))
                  .catch(() => {})
                  .finally(() => setBusy(false))
              }, 200)
            }}
          />
        </label>
        {(
          [
            ['edit.acne', 'acne'],
            ['edit.olhos', 'olhos'],
            ['edit.dentes', 'dentes'],
            ['edit.cabelo', 'cabelo']
          ] as Array<[string, string]>
        ).map(([labelKey, region]) => {
          const v = faceRegions[region] ?? 0
          return (
            <label key={region} className="edit-slider">
              <span>
                {t(labelKey)}
                <em>{Math.round(v * 100)}%</em>
              </span>
              <input
                type="range"
                min={0}
                max={100}
                step={1}
                value={Math.round(v * 100)}
                onChange={(e) => {
                  const val = Number(e.target.value) / 100
                  setFaceRegions((prev) => ({ ...prev, [region]: val }))
                  if (debounce.current) clearTimeout(debounce.current)
                  debounce.current = setTimeout(() => {
                    setBusy(true)
                    window.openshoot
                      .retouchFacePhoto(photo.id, region, val, 400)
                      .then((t) => t && setPreview(t))
                      .catch(() => {})
                      .finally(() => setBusy(false))
                  }, 200)
                }}
              />
            </label>
          )
        })}
      </div>

      <button onClick={applyCurrent} disabled={busy}>
        {t('edit.aplicarLote')}
      </button>
      <button onClick={removeDistraction} disabled={busy} className="ghost full">
        {t('edit.removerDistracao')}
      </button>

      <div className="edit-geo">
        <button
          onClick={autoLevel}
          disabled={busy}
          className="ghost full"
          title={t('edit.autoLevelHint')}
        >
          {t('edit.autoLevel')}
        </button>
        <button
          onClick={aiCrop}
          disabled={busy}
          className="ghost full"
          title={t('edit.aiCropHint')}
        >
          {t('edit.aiCrop')}
        </button>
      </div>

      <div className="edit-presets">
        <h4>{t('edit.presets')}</h4>
        <div className="edit-preset-save">
          <input
            type="text"
            placeholder={t('edit.presetNome')}
            value={presetName}
            onChange={(e) => setPresetName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') savePresetNow()
            }}
          />
          <button onClick={savePresetNow} disabled={!presetName.trim()} className="ghost">
            {t('edit.salvarPreset')}
          </button>
        </div>
        {presets.length === 0 ? (
          <p className="edit-hint">{t('edit.semPresets')}</p>
        ) : (
          <ul className="edit-preset-list">
            {presets.map((p) => (
              <li key={p.name}>
                <button className="edit-preset-load" onClick={() => applyRecipe(p.recipe)}>
                  {p.name}
                </button>
                <button
                  className="edit-preset-del"
                  title={t('dialog.deleteMoveTrash')}
                  onClick={() => removePreset(p.name)}
                >
                  ✕
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </aside>
  )
}
