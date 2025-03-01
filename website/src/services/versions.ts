import { downloadsServer } from './util'

export const LATEST = 'latest'

export async function listVersions(): Promise<string[]> {
    const response = await fetch(downloadsServer('/versions'))
    return await response.json()
}