{
    vars: {
        lint: {
            fix: false
        },
        publish: {
            version: '0.2.13',
            dryRun: false
        },
        runArgs: ['version'], 
        tests: null,
        rust: {
            channel: 'nightly'
        }
    },
    include: [
        { 
            path: '{{ root }}/user-variables.jsonnet', 
            optional: true 
        }
    ]
}