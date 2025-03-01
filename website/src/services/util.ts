export function downloadsServer(pathname: string): URL {
    const downloadsUrl = window.location.origin === 'http://localhost:3000'
        ? new URL('http://localhost:3001')
        : (() => {
            const url = new URL( window.location.origin)
            url.hostname = 'downloads.' + url.hostname
            return url
        })()
    if (typeof pathname === 'string')
        downloadsUrl.pathname = pathname
    return downloadsUrl
}


export function packageDownloadUrl(version: string, platform: string): URL {
    return downloadsServer(`/versions/${version}/builds/${platform}/package`)
}