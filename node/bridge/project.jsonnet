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
                    },
                    {
                        program: 'npm',
                        arguments: ['link', blaze.root + '/' + blaze.workspace.projects['node-devkit'].path]
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
            dependencies: ['node-devkit:build']
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
                path: 'npm-publish',
                pull: true
            },
            options: {
                releaseVersion: blaze.vars.publish.version,
                linkedDependencies: {
                    runtime: ['@blaze-repo/node-devkit']
                }
            },
            dependencies: [
                'build',
                'node-devkit:publish'
            ]
        }
    }
}