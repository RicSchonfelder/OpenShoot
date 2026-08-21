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
}

const EMPTY: EditValues = {
  exposure: null,
  temperature: null,
  tint: null,
  contrast: null,
  saturation: null,
  shadows: null,
  highlights: null,
  brightness: null
}

interface SliderDef {
  key: keyof EditValues
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

function toJson(values: EditValues): string {
  const o: Record<string, number> = {}
  for (const s of SLIDERS) {
    const v = values[s.key]
    if (v == null) continue
    const neutral = s.key === 'temperature' ? 6500 : 0
    if (v !== neutral) o[s.key] = v
  }
  return JSON.stringify(o)
}

interface EditPanelProps {
  photo: PhotoMeta | null
  onApplyAll: (json: string) => void
}

export default function EditPanel({ photo, onApplyAll }: EditPanelProps) {
  const { t } = useT()
  const [values, setValues] = useState<EditValues>(EMPTY)
  const [skinIntensity, setSkinIntensity] = useState(0)
  const [preview, setPreview] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const debounce = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Ao trocar de foto, carrega a receita salva (se houver).
  useEffect(() => {
    setValues(EMPTY)
    setPreview(null)
    if (!photo) return
    window.openshoot.getPhotoEdit(photo.id).then((json) => {
      if (!json) return
      try {
        const p = JSON.parse(json)
        setValues({
          exposure: p.exposure ?? null,
          temperature: p.temperature ?? null,
          tint: p.tint ?? null,
          contrast: p.contrast ?? null,
          saturation: p.saturation ?? null,
          shadows: p.shadows ?? null,
          highlights: p.highlights ?? null,
          brightness: p.brightness ?? null
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

  const onSlider = (key: keyof EditValues, val: number, neutral: number) => {
    const next = { ...values, [key]: val === neutral ? null : val }
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
      </div>

      <button onClick={applyCurrent} disabled={busy}>
        {t('edit.aplicarLote')}
      </button>
      <button onClick={removeDistraction} disabled={busy} className="ghost full">
        {t('edit.removerDistracao')}
      </button>
    </aside>
  )
}
