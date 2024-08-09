local cargo = (import 'cargo.libsonnet')();
local targets = import '../../targets.jsonnet';
local executors = import 'executors.libsonnet';
local blaze = std.extVar('blaze');

local workspaceDependencies = [
    { project: 'blaze-rust-devkit', crate: 'blaze-devkit' }, 
    { project: 'blaze-common', crate: 'blaze-common' }
];

local cargoTargets = cargo.all({
    workspaceDependencies: [dep.project for dep in workspaceDependencies]
});

{
    targets: cargoTargets + {
        publish: {
            executor: executors.cargoPublish(),
            options: {
                dryRun: blaze.vars.blaze.publish.dryRun
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
            executor: executors.cargoVersionCheck(),
            options: {
                version: blaze.vars.blaze.publish.version,
                workspaceDependencies: [dep.crate for dep in workspaceDependencies]
            }
        },
        ci: {
            dependencies: ['check', 'lint']
        }
    }
}