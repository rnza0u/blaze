local cargo = (import 'cargo.libsonnet')();
local executors = import 'executors.libsonnet';
local blaze = std.extVar('blaze');

local cargoTargets = cargo.all();

{
    targets: cargoTargets + {
        publish: {
            executor: executors.cargoPublish(),
            options: {
                dryRun: blaze.vars.blaze.publish.dryRun
            },
            dependencies: ['check-version']
        },
        'check-version': {
            executor: executors.cargoVersionCheck(),
            options: {
                version: blaze.vars.blaze.publish.version
            }
        },
        ci: {
            dependencies: ['lint', 'check']
        }
    }
}