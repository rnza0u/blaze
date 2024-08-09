local docusaurus = 'node_modules/.bin/docusaurus';
local docker = import 'docker.libsonnet';
local npm = import 'npm.libsonnet';
local env = {
    'ASSETS_LOCATION': '{{ workspace.root }}/{{ workspace.projects.blaze-assets.path }}',
    'PROJECT_ROOT': '{{ project.root }}'
};

{
    targets: {
        install: npm.install(),
        build: {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**',
                        'docs/**',
                        '*.js',
                        'tsconfig.json',
                    ],
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
                'install',
                'move-cli-docs',
                'move-json-schemas'
            ]
        },
        lint: npm.lint(),
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
                'install',
                'move-cli-docs',
                'move-json-schemas'
            ]
        },
        'move-json-schemas': {
            executor: 'std:commands',
            cache: {
                invalidateWhen: {
                    outputChanges: [
                        'docs/configuration/**/*-schema.json'
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
                            '{{ workspace.root }}/{{ workspace.projects.blaze-schemas.path }}/schemas/',
                            'static/'
                        ]
                    }
                ]
            },
            dependencies: [
                {
                    projects: ['blaze-schemas'],
                    target: 'build'
                }
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
                            '{{ workspace.root }}/{{ workspace.projects.blaze-cli-docs.path }}/dist/.',
                            'docs/cli'
                        ]
                    }
                ]
            },
            dependencies: [
                'blaze-cli-docs:build'
            ],
        },
        clean: npm.clean({
            unlinkPackage: '@blaze-repo/website', 
            extraDirectories: ['docs/cli', 'build', '.docusaurus']
        }),
        'build-image': docker.build('blaze-website', 'registry.rnzaou.me', ['conf/**']) + {
            dependencies: ['build']
        },
        'push-image': docker.push('blaze-website') + {
            dependencies: ['build-image', 'docker-registry:authenticate']
        },
        publish: {
            dependencies: [
                'push-image'
            ]
        },
        deploy: docker.composeUp(),
        ci: {
            dependencies: [
                'build-image',
                'lint'
            ]
        }
    }
}