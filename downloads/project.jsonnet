local image = 'registry.rnzaou.me/blaze-downloads';
local blaze = std.extVar('blaze');

local cargoArgs = (if blaze.vars.ci then ['--locked'] else []);

{
    targets: {
        'generate-lockfile': {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    inputChanges: ['Cargo.toml'],
                    outputChanges: ['Cargo.lock']
                }
            },
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
                    inputChanges: ['src/**', 'fixtures/**']
                }
            },
            dependencies: ['generate-lockfile']
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
                        arguments: cargoArgs + ['check']
                    },
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else [])
                    }
                ]
            },
            dependencies: ['source']
        },
        serve: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['run'],
                        environment: {
                            WEBSITE_ORIGIN: 'http://localhost:3000',
                            RUST_BACKTRACE: '1',
                            RUST_LOG: 'actix_web=debug',
                            LOG_LEVEL: 'debug',
                            BIN_ROOT: '{{ project.root }}/fixtures'
                        }
                    }
                ]
            },
            dependencies: ['source']
        },
        'build-bin': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cross',
                        arguments: cargoArgs + [
                            'build',
                            '--target', 
                            'x86_64-unknown-linux-musl',
                            '--release'
                        ]
                    }
                ]
            },
            dependencies: ['source']
        },
        'build-image': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'docker',
                        arguments: [
                            'build',
                            '-t',
                            image,
                            '.'
                        ]
                    }
                ]
            },
            dependencies: ['build-bin']
        },
        publish: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'docker',
                        arguments: [
                            'push',
                            image
                        ]
                    }
                ]
            },
            dependencies: ['build-image', 'ci:docker-authenticate']
        },
        deploy: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'docker',
                        arguments: [
                            'compose',
                            'up',
                            '--remove-orphans',
                            '--pull',
                            'always',
                            '--force-recreate',
                            '--detach'
                        ]
                    }
                ]
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