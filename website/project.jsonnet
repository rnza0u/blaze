local docusaurus = 'node_modules/.bin/docusaurus';
local image = 'registry.rnzaou.me/blaze-website';
local env = {
    'ASSETS_LOCATION': '{{ workspace.root }}/{{ workspace.projects.assets.path }}',
    'PROJECT_ROOT': '{{ project.root }}'
};
local blaze = std.extVar('blaze');

{
    targets: {
        install: {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    inputChanges: ['package.json'],
                    outputChanges: ['package-lock.json'],
                    filesMissing: ['node_modules']
                }
            },
            options: {
                commands: [
                    {
                        program: 'npm',
                        arguments: ['install']
                    }
                ]
            }
        },
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**',
                        'docs/**',
                        'static/**',
                        'tsconfig.json',
                        '*.js'
                    ]
                }
            },
            dependencies: [
                'install',
                'move-json-schemas',
                'move-cli-docs'
            ]
        },
        build: {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    outputChanges: [
                        'build/**'
                    ]
                }
            },
            options: {
                commands: [
                    {
                        program: docusaurus,
                        arguments: ['build'],
                        environment: env
                    }
                ]
            },
            dependencies: [
                'source'
            ]
        },
        lint: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: './node_modules/.bin/eslint',
                        arguments: (if blaze.vars.lint.fix then ['--fix'] else []) + [
                            blaze.project.root
                        ]
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
                        program: docusaurus,
                        arguments: ['start'],
                        environment: env
                    }
                ]
            },
            dependencies: [
                'source'
            ]
        },
        'move-json-schemas': {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    outputChanges: [
                        'static/schemas/**'
                    ]
                }
            },
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: [
                            '-rf',
                            'static/schemas'
                        ]
                    },
                    {
                        program: 'cp',
                        arguments: [
                            '--recursive',
                            '{{ workspace.root }}/{{ workspace.projects.schemas.path }}/schemas/',
                            'static/'
                        ]
                    }
                ]
            },
            dependencies: [
                'schemas:build'
            ]
        },
        'move-cli-docs': {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    outputChanges: [
                        'docs/cli/**'
                    ]
                }
            },
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: ['-rf', 'docs/cli']
                    },
                    {
                        program: 'mkdir',
                        arguments: ['-p', 'docs/cli']
                    },
                    {
                        program: 'cp',
                        arguments: [
                            '--recursive',
                            '{{ workspace.root }}/{{ workspace.projects.cli-docs.path }}/dist/.',
                            'docs/cli'
                        ]
                    }
                ]
            },
            dependencies: [
                'cli-docs:build'
            ],
        },
        clean: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: [
                            '-rf',
                            'build',
                            '.docusaurus',
                            'docs/cli',
                            'static/schemas',
                            'node_modules'
                        ]
                    }
                ]
            }
        },
        'build-image': {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'Dockerfile',
                        'conf/**'
                    ]
                }
            },
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
            dependencies: ['build']
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
            dependencies: ['build-image']
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
                            '--detach',
                            '--remove-orphans',
                            '--pull',
                            'always',
                            '--force-recreate'
                        ]
                    }
                ]
            }
        }
    }
}