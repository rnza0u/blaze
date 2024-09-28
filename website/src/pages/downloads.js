import Layout from '@theme/Layout'
import { useEffect, useState } from 'react'
import { filesize } from 'filesize'
import { listBuilds, listVersions, downloadUrl } from '../services/downloads'

const BUILD_STATE_LOADING = 'loading'
const BUILD_STATE_LOADED = 'loaded'
const BUILD_STATE_ERROR = 'error'

export default function Downloads() {
    const [availableVersions, setAvailableVersions] = useState(['latest'])
    const [selectedVersion, setSelectedVersion] = useState(availableVersions[0])
    const [builds, setBuilds] = useState({ state: BUILD_STATE_LOADING })

    function updateBuilds() {
        console.log(('updateBuilds'))
        setBuilds({ state: 'loading' })
        listBuilds(selectedVersion)
            .then(builds => setBuilds({ state: BUILD_STATE_LOADED, items: builds }))
            .catch(error => setBuilds({ state: BUILD_STATE_ERROR, error }))
    }

    function updateVersions() {
        listVersions()
            .then(versions => setAvailableVersions(['latest', ...versions]))
            .catch(err => {
                console.error(err)
            })
    }

    useEffect(() => updateBuilds(), [])
    useEffect(() => updateVersions(), [])

    return <Layout
        title={'Downloads'}
        description="Blaze downloads section.">
        <main role="main" style={{ padding: '32px', display: 'flex', flexDirection: 'column', justifyContent: 'center', alignItems: 'center' }}>
            <p>
                Are you looking for <a href="/docs/guides/get-started">
                    Blaze installation guidelines
                </a> ?
            </p>
            <label htmlFor="version">
                Selected version
            </label>
            <select id="version"
                className='margin-bottom--md'
                onChange={event => {
                    console.log('onChange')
                    setSelectedVersion(event.target.value)
                    updateBuilds()
                }}>
                {availableVersions.map(version =>
                    <option key={version}
                        value={version}
                        defaultValue={selectedVersion === version}
                    >
                        {version}
                    </option>
                )}
            </select>
            <table style={{ textAlign: 'center' }}>
                <caption>Available downloads</caption>
                <thead>
                    <tr>
                        <th scope="col">Version</th>
                        <th scope="col">Platform</th>
                        <th scope="col">Checksum</th>
                        <th scope="col">Size</th>
                        <th scope="col">Download</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        (() => {
                            switch (builds.state) {
                                case BUILD_STATE_LOADING:
                                    return <tr>
                                        <td colSpan="5" style={{ textAlign: 'center' }}>
                                            Loading...
                                        </td>
                                    </tr>
                                case BUILD_STATE_LOADED:
                                    return Object.entries(builds.items).map(([platform, build]) =>
                                        <tr key={platform}>
                                            <td>{build.version}</td>
                                            <td>{platform}</td>
                                            <td>{build.checksum} (sha256)</td>
                                            <td>{filesize(build.size)}</td>
                                            <td>
                                                <a href={downloadUrl(build.version, platform).toString()}>
                                                    <i className="w-icon-download margin-right--xs"></i>
                                                </a>
                                            </td>
                                        </tr>
                                    )
                                case BUILD_STATE_ERROR:
                                    return <tr>
                                        <td colSpan="5" style={{ textAlign: 'center', color: 'var(--ifm-color-danger-dark)' }}>
                                            An error occured ({builds.error.message})
                                        </td>
                                    </tr>
                            }
                        })()
                    }
                </tbody>
            </table>
        </main>
    </Layout>
}