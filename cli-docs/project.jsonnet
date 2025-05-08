local blaze = std.extVar('blaze');
local targets = import '../targets.jsonnet';
local LocalEnv = import '../core/local-env.jsonnet';
local workspaceDependencies = [
    { crate: 'blaze-cli', project: 'cli' }
];
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
                    inputChanges: [
                        'src/**'
                    ]
                }
            },
            dependencies: [
                {
                    projects: [dep.project for dep in workspaceDependencies],
                    target: 'source'
                },
                'generate-lockfile'
            ]
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
        },
        lint: {
            executor: 'std:commands',
            options: {
                commands: (if blaze.vars.lint.fix then [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['fmt'],
                        environment: LocalEnv(targets.dev)
                    }
                ] else []) + [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['check'],
                        environment: LocalEnv(targets.dev)
                    },
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else []),
                        environment: LocalEnv(targets.dev)
                    }
                ]
            },
            dependencies: [
                'source'
            ]
        },
        build: {
            executor: 'std:commands',
            description: 'Build the documentation files.',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + [
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