export interface PhotoMeta {
  id: number
  path: string
  fileName: string
  ext: string
  fileSize: number
  width: number
  height: number
  camera: string
  takenAt: string | null
  rating: number
  hasXmp: boolean
  previewAvailable: boolean
  cullScore: number | null
  hash: string
  hasFace: boolean
  review: boolean
  aiPick: boolean
}

export interface ScanResultData {
  scanned: number
  added: number
  updated: number
  skipped: number
  errors: string[]
}

export interface PhotoListData {
  photos: PhotoMeta[]
  total: number
}
