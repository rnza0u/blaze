local blaze = std.extVar('blaze');

{
    vars: {
        lint: {
            fix: false
        },
        publish: {
            version: '0.2.16'
        },
        runArgs: ['version'], 
        tests: null,
        ci: std.objectHas(blaze.environment, 'CI') && blaze.environment.CI == "true"
    },
    include: [
        { 
            path: '{{ root }}/user-variables.jsonnet', 
            optional: true 
        }
    ]
}