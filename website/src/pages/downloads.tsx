import Layout from '@theme/Layout'
import { filesize } from 'filesize'
import { packageDownloadUrl } from '../services/util'
import React from 'react'
import { useVersions } from '../hooks/versions'
import { useBuilds } from '../hooks/builds'
import { LATEST } from '../services/versions'
import style from './downloads.module.css'

export default function Downloads() {
    const versions = useVersions()
    const { loadVersion, ...builds } = useBuilds(LATEST)

    return <Layout
        title={'Downloads'}
        description="Blaze downloads section.">
        <main role="main" className={style.container}>
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
                onChange={event => loadVersion(event.target.value)}
            >
                {versions.status === 'ready' && versions.versions.map((version) =>
                    <option key={version} value={version}>
                        {version}
                    </option>
                )}
            </select>
            <table className={style.table}>
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
                            switch (builds.status) {
                                case 'loading':
                                    return <tr>
                                        <td colSpan={5} className={style.loading}>
                                            Loading...
                                        </td>
                                    </tr>
                                case 'ready':
                                    return Object.entries(builds.builds).map(([platform, build]) =>
                                        <tr key={platform}>
                                            <td>{build.version}</td>
                                            <td>{platform}</td>
                                            <td>{build.checksum} (sha256)</td>
                                            <td>{filesize(build.size)}</td>
                                            <td>
                                                <a href={packageDownloadUrl(build.version, platform).toString()}>
                                                    <span className='sr-only'>Download</span>
                                                    <i className="w-icon-download margin-right--xs"></i>
                                                </a>
                                            </td>
                                        </tr>
                                    )
                                case 'error':
                                    return <tr>
                                        <td colSpan={5} className={style.row}>
                                            An error occured ({`${builds.error}`})
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