import type { OpenShootApi } from '../preload'

declare global {
  interface Window {
    openshoot: OpenShootApi
  }
}

export {}
