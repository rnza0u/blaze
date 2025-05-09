import { useEffect, useState } from 'react'
import { listVersions } from '../services/versions'
import semver from 'semver'

export function useLatestVersion(): string | undefined {
    const [latestVersion, setLatestVersion] = useState<string | undefined>()
    useEffect(() => {
        listVersions()
            .then((versions) => {
                if (versions.length === 0) {
                    throw Error('no versions available')
                }
                const sorted = semver.sort(
                    versions.map((version) => semver.parse(version)!),
                )
                setLatestVersion(sorted[versions.length - 1].toString())
            })
            .catch((err) => {
                console.error(err)
            })
    }, [])
    return latestVersion
}
