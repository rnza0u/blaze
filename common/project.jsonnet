local blaze = std.extVar('blaze');

local cargoArgs = (if blaze.vars.ci then ['--locked'] else []);

{
    targets: {
        'generate-lockfile': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['generate-lockfile']
                    }
                ]
            }
        },
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**',
                        'Cargo.toml'
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
                        arguments: cargoArgs + ['fmt']
                    }
                ] else []) + [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else [])
                    },
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['check']
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
                        arguments: cargoArgs + ['clean']
                    }
                ]
            }
        }
    }
}