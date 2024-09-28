local blaze = std.extVar('blaze');

{
    targets: {
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**',
                        'Cargo.toml'
                    ],
                    outputChanges: ['Cargo.lock']
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
                        arguments: ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else [])
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
                format: 'Git',
                pull: true
            },
            options: {
                releaseVersion: blaze.vars.publish.version
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