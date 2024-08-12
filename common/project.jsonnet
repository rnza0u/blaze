local blaze = std.extVar('blaze');

{
    targets: {
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**',
                        'Cargo.toml',
                        'Cargo.lock'
                    ]
                }
            }
        },
        lint: {
            executor: 'std:commands',
            options: {
                commands: (if blaze.vars.lint.fix then [
                    {
                        program: 'cargo',
                        arguments: ['fmt']
                    }
                ] else []) + [
                    {
                        program: 'cargo',
                        arguments: ['clippy']
                    },
                    {
                        program: 'cargo',
                        arguments: ['check']
                    }
                ]
            }
        },
        publish: {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'cargo-publish',
                format: 'Git'
            },
            options: {
                dryRun: blaze.vars.publish.dryRun
            },
            dependencies: ['check-version']
        },
        'check-version': {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'cargo-version-check',
                format: 'Git'
            },
            options: {
                version: blaze.vars.publish.version
            }
        },
        clean: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: ['clean']
                    }
                ]
            }
        }
    }
}