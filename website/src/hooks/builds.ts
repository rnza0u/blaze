import { useEffect, useMemo, useState } from 'react'

import { downloadsServer } from '../services/util'

export type Build = Readonly<{
    checksum: string;
    version: string;
    size: number;
}>

export async function listBuilds(version: string): Promise<Build[]> {
    const response = await fetch(downloadsServer(`/versions/${version}/builds`))
    return await response.json()
}

type BuildStateStatus =
  | Readonly<{
      status: 'loading';
  }>
  | Readonly<{
      status: 'error';
      error: unknown;
  }>
  | Readonly<{
      status: 'ready';
      builds: readonly Build[];
  }>

type BuildsState =
  & BuildStateStatus
  & Readonly<{
      loadVersion(version: string): void;
  }>

export function useBuilds(initialVersion: string): BuildsState {
    const abortController = useMemo(() => new AbortController(), [])

    const [builds, setBuilds] = useState<BuildStateStatus>({
        status: 'loading',
    })

    function load(version: string): void {
        setBuilds({ status: 'loading' })
        fetch(downloadsServer(`/versions/${version}/builds`), {
            signal: abortController.signal,
        })
            .then((response) => response.json())
            .then((builds) =>
                setBuilds({
                    status: 'ready',
                    builds: builds,
                })
            )
            .catch((error) => {
                if (error instanceof DOMException && error.name === 'AbortError') {
                    return
                }
                setBuilds({ status: 'error', error })
            })
    }

    useEffect(() => load(initialVersion), [])

    return {
        ...builds,
        loadVersion: (version) => {
            abortController.abort()
            load(version)
        },
    }
}
