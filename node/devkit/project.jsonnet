
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
                    filesMissing: ['node_modules'],
                    inputChanges: ['package.json'],
                    outputChanges: ['package-lock.json']
                }
            }
        },
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: ['src/**', 'tsconfig.json']
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
                    outputChanges: [
                        'lib/**'
                    ]
                }
            },
            dependencies: ['source']
        },
        publish: {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                format: 'Git',
                path: 'npm-publish',
                pull: true
            },
            options: {
                releaseVersion: blaze.vars.publish.version
            },
            dependencies: ['build']
        },
        clean: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: ['-rf', 'node_modules', 'lib']
                    },
                    {
                        program: 'npm',
                        arguments: ['uninstall', '--global', '@blaze-repo/node-devkit']
                    }
                ]
            }
        }
    }
}