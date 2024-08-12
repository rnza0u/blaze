local blaze = std.extVar('blaze');

{
    targets: {
        install: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'npm',
                        arguments: ['install']
                    }
                ]
            },
            cache: {
                invalidateWhen: {
                    inputChanges: ['package.json'],
                    outputChanges: ['package-lock.json'],
                    filesMissing: ['node_modules']
                }
            },
            dependencies: ['blaze-node-devkit:link']
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
            dependencies: [
                'source'
            ]
        },
        build: {
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
        clean: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: ['-rf', 'dist', 'node_modules']
                    }
                ]
            }
        },
        publish: {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                format: 'Git',
                path: 'npm-publish'
            },
            options: {
                dryRun: blaze.vars.publish.dryRun
            },
            dependencies: [
                'build',
                'check-version',
                'blaze-node-devkit:publish'
            ]
        },
        'check-version': {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                format: 'Git',
                path: 'npm-check-version'
            },
            options: {
                version: blaze.vars.publish.version,
                workspaceDependencies: [
                    '@blaze-repo/node-devkit'
                ]
            }
        }
    }
}