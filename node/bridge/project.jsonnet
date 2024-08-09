local npm = import 'npm.libsonnet';
local executors = import 'executors.libsonnet';
local blaze = std.extVar('blaze');

{
    targets: npm.all(
        '@blaze-repo/node-bridge', 
        {
            workspaceDependencies: ['blaze-node-devkit']
        }
    ) + {
        publish: {
            executor: executors.npmPublish(),
            options: {
                dryRun: blaze.vars.blaze.publish.dryRun
            },
            dependencies: [
                'build',
                'check-version',
                'blaze-node-devkit:publish'
            ]
        },
        'check-version': {
            executor: executors.npmVersionCheck(),
            options: {
                version: blaze.vars.blaze.publish.version,
                workspaceDependencies: [
                    '@blaze-repo/node-devkit'
                ]
            }
        },
        ci: {
            dependencies: ['lint', 'build']
        }
    }
}