import { useEffect, useState } from 'react'
import { listVersions } from '../services/versions'

const LATEST = 'latest'

type VersionsState = 
  (|Readonly<{
      status: 'loading'
  }>
  |Readonly<{
      status: 'error',
      error: unknown
  }>
  |Readonly<{
      status: 'ready'
  }>) & Readonly<{
      versions: readonly string[]
  }>

export function useVersions(){
    const [state, setState] = useState<VersionsState>({
        status: 'loading',
        versions: [LATEST]
    })

    useEffect(() => {
        listVersions()
            .then(versions => setState({
                status: 'ready',
                versions: [LATEST, ...versions]
            }))
            .catch(error => {
                console.error(error)
                setState(({ versions }) => ({
                    error,
                    status: 'error',
                    versions
                })) 
            })
    }, [])

    return state
}