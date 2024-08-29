local blaze = std.extVar('blaze');
local LocalEnv = import '../core/local-env.jsonnet';
local targets = import '../targets.jsonnet';
local finalTargets = std.filter(function(name) targets[name].rustTriple != null, std.objectFields(targets));

local workspaceDependencies = [{ project: 'core', crate: 'blaze-core' }];

local testTargets = {
    ['run-' + name]: {
        local useCross = targets[name].rustTriple != null,
        executor: 'std:commands',
        cache: {
            invalidateWhen: {
                inputChanges: [
                    {
                        pattern: 'tests/**',
                        exclude: ['**/node_modules', '**/target']
                    }
                ]
            }
        },
        options: {
            commands: [
                {
                    program: if useCross then 'cross' else 'cargo',
                    arguments: [
                        '+nightly',
                        'test', 
                        '--no-fail-fast'
                    ] + (if blaze.vars.tests != null 
                        then std.foldl(
                            function(testArgs, test) testArgs + ['--test', test], 
                            blaze.vars.tests, 
                            []
                        ) 
                        else []
                    )
                    + (if targets[name].rustTriple != null then ['--target',  targets[name].rustTriple] else [])
                    + (if targets[name].release then ['--release'] else [])
                    + ['--', '--nocapture'],
                    environment: LocalEnv(targets[name])
                    + (if useCross then { 
                        BLAZE_ROOT: blaze.root,
                        CROSS_CONFIG: blaze.root + '/Cross.toml' 
                    } else {})
                },
            ]
        },
        dependencies: [
            'source'
        ] + (if useCross then ['ci:docker-authenticate'] else [])
    } for name in std.objectFields(targets)
};

{
    targets: testTargets + {
        source: {
            cache: {},
            dependencies: [
                {
                    projects: [dep.project for dep in workspaceDependencies],
                    target: 'source'
                }
            ]
        },
        publish: {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'cargo-publish',
                format: 'Git'
            },
            options: {
                dryRun: blaze.vars.publish.dryRun,
                channel: 'nightly'
            },
            dependencies: [
                'check-version',
                {
                    projects: [dep.project for dep in workspaceDependencies],
                    target: 'publish'
                }
            ]
        },
        'check-version': {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                path: 'cargo-version-check',
                format: 'Git'
            },
            options: {
                version: blaze.vars.publish.version,
                workspaceDevDependencies: [dep.crate for dep in workspaceDependencies]
            }
        },
        run: {
            cache: {},
            dependencies: ['run-dev']
        },
        'run-all': {
            cache: {},
            dependencies: ['run-' + target for target in finalTargets]
        },
        lint: {
            executor: 'std:commands',
            options: {
                commands: (if blaze.vars.lint.fix then [
                    {
                        program: 'cargo',
                        arguments: ['fmt']
                    }
                ] else []) + [
                    {
                        program: 'cargo',
                        arguments: ['check']
                    },
                    {
                        program: 'cargo',
                        arguments: ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else [])
                    }
                ]
            },
            dependencies: ['source']
        },
        'clean-fixtures': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: [
                            '-rf',
                            'dist',
                            'node_modules',
                        ],
                        cwd: blaze.project.root + '/tests/fixtures/executors/node-checker'
                    },
                    {
                        program: 'cargo',
                        arguments: ['clean'],
                        cwd: blaze.project.root + '/tests/fixtures/executors/rust-checker'
                    }
                ]
            }
        },
        'clean-cargo': {
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
        clean: {
            dependencies: [
                'clean-cargo',
                'clean-fixtures'
            ]
        }
    }
}