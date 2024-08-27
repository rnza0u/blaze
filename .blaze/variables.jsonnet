{
    vars: {
        lint: {
            fix: false
        },
        publish: {
            version: '0.2.11',
            dryRun: false
        },
        runArgs: ['version'], 
        tests: null,
        rust: {
            channel: 'nightly-2024-06-25'
        }
    },
    include: [
        '{{ root }}/user-variables.jsonnet'
    ]
}