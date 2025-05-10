
local blaze = std.extVar('blaze');

{
    targets: {
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**', 
                        'tsconfig.json'
                    ]
                }
            },
            dependencies: ['pnpm:install']
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
                    './node_modules/.bin/tsc'
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
                path: 'pnpm-publish',
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
                        arguments: ['-rf', 'lib']
                    }
                ]
            }
        }
    }
}