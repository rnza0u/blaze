local blaze = std.extVar('blaze');

{
    targets: {
        install: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'npm',
                        arguments: ['ci']
                    }
                ]
            },
            cache: {
                invalidateWhen: {
                    inputChanges: ['package.json'],
                    outputChanges: ['package-lock.json'],
                    filesMissing: ['node_modules']
                }
            }
        },
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**', 
                        'tsconfig.json'
                    ]
                }
            },
            dependencies: ['install']
        },
        lint: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: './node_modules/.bin/eslint',
                        arguments: (if blaze.vars.lint.fix then ['--fix'] else [])
                            + [blaze.project.root]
                    }
                ]
            },
            dependencies: ['source']
        },
        'build-generator': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'npm',
                        arguments: ['run', 'build']
                    }
                ]
            },
            cache: {
                invalidateWhen: {
                    outputChanges: ['dist/**']
                }
            },
            dependencies: ['source']
        },
        build: {
            executor: 'std:commands',
            description: 'Build the JSON schemas.',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: ['-rf', 'schemas']
                    },
                    {
                        program: 'mkdir',
                        arguments: ['-p', 'schemas']
                    },
                    {
                        program: 'npm',
                        arguments: ['start']
                    }
                ]
            },
            cache: {
                invalidateWhen: {
                    outputChanges: [
                        'schemas/**'
                    ]
                }
            },
            dependencies: [
                'build-generator'
            ]
        },
        publish: {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'npm-publish',
                format: 'Git'
            },
            options: {
                dryRun: blaze.vars.publish.dryRun
            },
            dependencies: ['check-version', 'build']
        },
        'check-version': {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'npm-version-check',
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
                        program: 'rm',
                        arguments: [
                            '-rf', 
                            'schemas', 
                            'dist', 
                            'node_modules'
                        ]
                    }
                ]
            }
        }
    }
}