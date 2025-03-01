import { downloadsServer } from './util'

export type Build = Readonly<{
  checksum: string
  version: string
  size: number
}>

export async function listBuilds(version): Promise<Build[]> {
    const response = await fetch(downloadsServer(`/versions/${version}/builds`))
    return await response.json()
}