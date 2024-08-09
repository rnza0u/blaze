local cargo = (import 'cargo.libsonnet')();
local docker = import 'docker.libsonnet';

local cargoTargets = cargo.all();

{
    targets: cargoTargets + {
        serve: {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cargo',
                        arguments: ['run'],
                        environment: {
                            WEBSITE_ORIGIN: 'http://localhost:3000',
                            RUST_BACKTRACE: '1',
                            RUST_LOG: 'actix_web=debug',
                            LOG_LEVEL: 'debug',
                            BIN_ROOT: '{{ project.root }}/fixtures'
                        }
                    }
                ]
            }
        },
        'build-bin': {
            executor: 'std:commands',
            options: {
                commands: [
                    {
                        program: 'cross',
                        arguments: [
                            'build',
                            '--target', 
                            'x86_64-unknown-linux-musl',
                            '--release'
                        ]
                    }
                ]
            },
            dependencies: ['source']
        },
        'build-image': docker.build('blaze-downloads') + {
            dependencies: ['build-bin']
        },
        'push-image': docker.push('blaze-downloads') + {
            dependencies: ['build-image']
        },
        deploy: docker.composeUp(),
        publish: {
            dependencies: ['push-image']
        },
        ci: {
            dependencies: ['check', 'lint']
        }
    }
}