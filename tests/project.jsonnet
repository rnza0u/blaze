local blaze = std.extVar('blaze');
local LocalEnv = import '../core/local-env.jsonnet';
local targets = import '../targets.jsonnet';
local cargo = (import 'cargo.libsonnet')(blaze.vars.blaze.rust.channel, ['-Z', 'bindeps']);
local executors = import 'executors.libsonnet';

local workspaceDependencies = ['blaze-core'];

local cargoTargets = cargo.all({
    workspaceDependencies: workspaceDependencies
});

{
    targets: cargoTargets + {
        ['test-' + name]: {
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
                        local useCross = targets[name].rustTriple != null,
                        program: if useCross then 'cross' else 'cargo',
                        arguments: [
                            '+nightly',
                            'test', 
                            '--no-fail-fast'
                        ] + (if blaze.vars.blaze.tests != null 
                            then std.foldl(
                                function(testArgs, test) testArgs + ['--test', test], 
                                blaze.vars.blaze.tests, 
                                []
                            ) 
                            else []
                        )
                        + (if targets[name].rustTriple != null then ['--target',  targets[name].rustTriple] else [])
                        + (if targets[name].release then ['--release'] else [])
                        + ['--', '--nocapture'],
                        environment: LocalEnv(targets[name])
                        + (if useCross then { 
                            BLAZE_ROOT: blaze.root + '/blaze',
                            CROSS_CONFIG: blaze.root + '/blaze/Cross.toml' 
                        } else {})
                    },
                ]
            },
            dependencies: [
                'source'
            ]
        } for name in std.objectFields(targets)
    } + {
        publish: {
            executor: executors.cargoPublish(),
            options: {
                dryRun: blaze.vars.blaze.publish.dryRun,
                channel: 'nightly'
            },
            dependencies: [
                'check-version',
            ] + [dep + ':publish' for dep in workspaceDependencies]
        },
        'check-version': {
            executor: executors.cargoVersionCheck(),
            options: {
                version: blaze.vars.blaze.publish.version,
                workspaceDevDependencies: workspaceDependencies
            }
        },
        test: {
            cache: {},
            dependencies: ['test-dev']
        },
        check: cargo.check(),
        lint: cargo.lint(),
        'clean-fixtures': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'rm',
                        arguments: [
                            '-rf',
                            'tests/fixtures/executors/node-checker/dist',
                            'tests/fixtures/executors/node-checker/node_modules',
                            'tests/fixtures/executors/rust-dummy/target',
                            'tests/fixtures/executors/rust-checker/target'
                        ]
                    }
                ]
            }
        },
        'clean-cargo': cargoTargets.clean,
        clean: {
            dependencies: [
                'clean-cargo',
                'clean-fixtures'
            ]
        }
    }
}