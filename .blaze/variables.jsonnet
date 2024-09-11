{
    vars: {
        lint: {
            fix: false
        },
        publish: {
            version: '0.2.15'
        },
        runArgs: ['version'], 
        tests: null
    },
    include: [
        { 
            path: '{{ root }}/user-variables.jsonnet', 
            optional: true 
        }
    ]
}