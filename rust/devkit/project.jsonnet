local executors = import 'executors.libsonnet';
local cargo = (import 'cargo.libsonnet')();
local blaze = std.extVar('blaze');

local workspaceDependencies = ['blaze-common'];

local cargoTargets = cargo.all({
    workspaceDependencies: workspaceDependencies
});

{
    targets: cargoTargets + {
        publish: {
            executor: executors.cargoPublish(),
            options: {
                dryRun: blaze.vars.blaze.publish.dryRun
            },
            dependencies: [
                'check-version'
            ] + [dep + ':publish' for dep in workspaceDependencies]
        },
        'check-version': {
            executor: executors.cargoVersionCheck(),
            options: {
                version: blaze.vars.blaze.publish.version,
                workspaceDependencies: workspaceDependencies
            }
        },
        ci: {
            dependencies: ['check', 'lint']
        }
    }
}