import { useEffect, useState } from 'react'
import { listVersions } from '../services/versions'

const LATEST = 'latest'

type BuildsState = 
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
  const [state, setState] = useState<BuildsState>({
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
        setState(({ versions }) => ({
          error,
          status: 'error',
          versions
        })) 
      })
  }, [])

  return state
}