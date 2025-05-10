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
            dependencies: ['pnpm:install', 'node-devkit:build']
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
                    './node_modules/.bin/tsc',
                    './node_modules/.bin/esbuild dist/main.js --bundle --outfile=dist/main.js --platform=node --minify --allow-overwrite=true --format=esm'
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
                        arguments: ['-rf', 'dist']
                    }
                ]
            }
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
            dependencies: [
                'build',
                'node-devkit:publish'
            ]
        }
    }
}