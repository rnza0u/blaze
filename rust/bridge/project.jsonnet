local targets = import '../../targets.jsonnet';
local blaze = std.extVar('blaze');

local workspaceDependencies = [
    { project: 'rust-devkit', crate: 'blaze-devkit' }, 
    { project: 'common', crate: 'blaze-common' }
];

{
    targets: {
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**', 
                        'Cargo.toml', 
                        'Cargo.lock'
                    ]
                }
            },
            dependencies: [
                {
                    projects: [dep.project for dep in workspaceDependencies],
                    target: 'source'
                }
            ]
        },
        lint: {
            executor: 'std:commands',
            cache: {},
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
        publish: {
            executor: {
                url: 'https://github.com/rnza0u/blaze-executors.git',
                format: 'Git',
                path: 'cargo-publish',
                pull: true
            },
            options: {
                releaseVersion: blaze.vars.publish.version,
                linkedDependencies: {
                    runtime: [dep.crate for dep in workspaceDependencies]
                }
            },
            dependencies: [
                {
                    projects: [dep.project for dep in workspaceDependencies],
                    target: 'publish'
                }
            ]
        },
        clean: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: ['clean']
                    }
                ]
            }
        }
    }
}