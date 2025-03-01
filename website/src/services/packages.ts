import { downloadsServer } from './util'

export function packageDownloadUrl(version: string, platform: string): URL {
  return downloadsServer(`/versions/${version}/builds/${platform}/package`)
}