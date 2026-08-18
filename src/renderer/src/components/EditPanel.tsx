import { useCallback, useEffect, useRef, useState } from 'react'
import type { PhotoMeta } from '../../../types/photo'

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
  label: string
  min: number
  max: number
  step: number
  unit: string
}

const SLIDERS: SliderDef[] = [
  { key: 'exposure', label: 'Exposição', min: -3, max: 3, step: 0.05, unit: 'EV' },
  { key: 'temperature', label: 'Temperatura', min: 2000, max: 12000, step: 50, unit: 'K' },
  { key: 'tint', label: 'Tint', min: -100, max: 100, step: 1, unit: '' },
  { key: 'contrast', label: 'Contraste', min: -100, max: 100, step: 1, unit: '' },
  { key: 'saturation', label: 'Saturação', min: -100, max: 100, step: 1, unit: '' },
  { key: 'shadows', label: 'Sombras', min: -100, max: 100, step: 1, unit: '' },
  { key: 'highlights', label: 'Realces', min: -100, max: 100, step: 1, unit: '' },
  { key: 'brightness', label: 'Brilho', min: -100, max: 100, step: 1, unit: '' }
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
  const [values, setValues] = useState<EditValues>(EMPTY)
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

  if (!photo) {
    return (
      <aside className="edit-panel">
        <h3>Edição</h3>
        <p className="edit-hint">Selecione uma foto para editar.</p>
      </aside>
    )
  }

  return (
    <aside className="edit-panel">
      <h3>Edição — {photo.fileName}</h3>
      <div className="edit-preview">
        {preview ? (
          <img src={preview} alt="preview editado" />
        ) : (
          <div className="edit-preview-empty">{busy ? 'Processando…' : 'Ajuste os controles'}</div>
        )}
      </div>
      <div className="edit-sliders">
        {SLIDERS.map((s) => {
          const neutral = s.key === 'temperature' ? 6500 : 0
          const v = values[s.key] ?? neutral
          return (
            <label key={s.key} className="edit-slider">
              <span>
                {s.label}
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
      <button onClick={applyCurrent} disabled={busy}>
        Aplicar em lote
      </button>
    </aside>
  )
}
