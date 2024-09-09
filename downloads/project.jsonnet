local image = 'registry.rnzaou.me/blaze-downloads';
local blaze = std.extVar('blaze');

{
    targets: {
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
                        arguments: ['check']
                    },
                    {
                        program: 'cargo',
                        arguments: ['clippy']
                    }
                ]
            }
        },
        serve: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: ['run'],
                        environment: {
                            WEBSITE_ORIGIN: 'http://localhost:3000',
                            RUST_BACKTRACE: '1',
                            RUST_LOG: 'actix_web=debug',
                            LOG_LEVEL: 'debug',
                            BIN_ROOT: '{{ project.root }}/fixtures'
                        }
                    }
                ]
            }
        },
        'build-bin': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cross',
                        arguments: [
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
                        arguments: ['clean']
                    }
                ]
            }
        }
    }
}