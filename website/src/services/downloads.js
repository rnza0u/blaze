function downloadsServer(path) {
    const currentUrl = new URL(window.location.href)
    const downloadsUrl = currentUrl.origin === new URL('http://localhost:3000').origin
        ? new URL('http://localhost:3001')
        : (() => {
            const url = new URL(currentUrl)
            url.hostname = 'downloads.' + url.hostname
            return url
        })()
    if (typeof path === 'string') {
        downloadsUrl.pathname = path
    }
    return downloadsUrl
}

export function listBuilds(version) {
    return fetch(downloadsServer(`/versions/${version}/builds`))
        .then(response => response.json())
}

export function listVersions() {
    return fetch(downloadsServer('/versions'))
        .then(response => response.json())
}

export function downloadUrl(version, platform) {
    return downloadsServer(`/versions/${version}/builds/${platform}/package`)
}