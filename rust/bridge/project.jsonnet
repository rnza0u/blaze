local targets = import '../../targets.jsonnet';
local blaze = std.extVar('blaze');

local workspaceDependencies = [
    { project: 'rust-devkit', crate: 'blaze-devkit' }, 
    { project: 'common', crate: 'blaze-common' }
];

local cargoArgs = (if blaze.vars.ci then ['--locked'] else []);

{
    targets: {
        'generate-lockfile': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['generate-lockfile']
                    }
                ]
            }
        },
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'src/**',
                        'Cargo.toml'
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
                        arguments: cargoArgs + ['fmt']
                    }
                ] else []) + [
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['check']
                    },
                    {
                        program: 'cargo',
                        arguments: cargoArgs + ['clippy', '--no-deps'] + (if blaze.vars.lint.fix then ['--fix', '--allow-dirty'] else [])
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
                        arguments: cargoArgs + ['clean']
                    }
                ]
            }
        }
    }
}