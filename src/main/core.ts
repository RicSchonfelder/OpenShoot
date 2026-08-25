import { createRequire } from 'node:module'
import { join } from 'node:path'
import { app } from 'electron'

const require = createRequire(__filename)
let core: typeof import('*.node')

export function loadCore(): void {
  const base = join(app.getAppPath(), 'core')
  const candidates = [
    join(base, `openshoot_core.${process.platform}.${process.arch}.node`),
    join(base, 'openshoot_core.node')
  ]
  const found = candidates.find((p) => {
    try {
      require.resolve(p)
      return true
    } catch {
      return false
    }
  })
  if (!found) {
    throw new Error(
      'OpenShoot core (.node) nao encontrado. Rode: npm run build:core'
    )
  }
  core = require(found) as typeof import('*.node')
}

export function getCore(): typeof import('*.node') {
  if (!core) loadCore()
  return core
}
