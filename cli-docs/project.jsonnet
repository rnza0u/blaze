local blaze = std.extVar('blaze');
local targets = import '../targets.jsonnet';
local LocalEnv = import '../core/local-env.jsonnet';
local workspaceDependencies = ['blaze-cli'];

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
            },
            dependencies: [
                {
                    projects: workspaceDependencies,
                    target: 'source'
                }
            ]
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
        },
        lint: {
            executor: 'std:commands',
            options: {
                commands: (if blaze.vars.lint.fix then [
                    {
                        program: 'cargo',
                        arguments: ['fmt'],
                        environment: LocalEnv(targets.dev)
                    }
                ] else []) + [
                    {
                        program: 'cargo',
                        arguments: ['check'],
                        environment: LocalEnv(targets.dev)
                    },
                    {
                        program: 'cargo',
                        arguments: ['clippy'],
                        environment: LocalEnv(targets.dev)
                    }
                ]
            },
            dependencies: [
                {
                    projects: workspaceDependencies,
                    target: 'source'
                }
            ]
        },
        build: {
            executor: 'std:commands',
            description: 'Build the documentation files.',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: [
                            'run', 
                            '--release'
                        ],
                        environment: LocalEnv(targets.release) + {
                            OUT_DIR: '{{ project.root }}/dist'
                        }
                    }
                ],
            },
            cache: {
                invalidateWhen: {
                    outputChanges: ['dist/**']
                }
            },
            dependencies: [
                'source'
            ]
        }
    }
}