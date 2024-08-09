import React, { useEffect, useState } from 'react'
import { listVersions } from '../../services/downloads'
import semver from 'semver'

export default function (props){
    const [latestVersion, setLatestVersion] = useState('<version number>')
    useEffect(() => {
        listVersions()
            .then(versions => {
                if (versions.length === 0){
                    throw Error('no versions available')
                }
                versions = semver.sort(versions.map(version => semver.parse(version)))
                setLatestVersion(versions[versions.length - 1].toString())
            })
            .catch(err => {
                console.error(err)
            })
    }, [setLatestVersion])

    return <>
        {props.children(latestVersion)}
    </>
}