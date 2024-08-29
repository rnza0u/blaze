local cacheEnv = {
    CACHE_STORAGE: '/var/lib/cache',
    CACHE_KEY: 'blaze-ci-${DRONE_BRANCH}',
    CACHE_EXTRA_DIRS: std.join(',', [
        "/root/.cargo",
        "/root/.npm",
        "/root/.rustup",
        ".blaze/cache",
        ".blaze/repositories",
        ".blaze/rust"
    ])
};

local cacheVolumes = [
    {
        name: 'cache',
        path: '/var/lib/cache',
    },
];

local dockerVolumes = [
    {
        name: 'docker-socket',
        path: '/var/run/docker.sock',
    }
];

local dockerCredentials = {
    DOCKER_REGISTRY_USERNAME: {
        from_secret: 'DOCKER_REGISTRY_USERNAME'
    },
    DOCKER_REGISTRY_PASSWORD: {
        from_secret: 'DOCKER_REGISTRY_PASSWORD'
    }
};

local Step = function(config){
    image: 'registry.rnzaou.me/ci:latest',
    pull: 'always'
} + config;

local targets = import '../targets.jsonnet';
local finalTargets = std.filter(function(name) targets[name].rustTriple != null, std.objectFields(targets));

local ci = {
  kind: 'pipeline',
  type: 'docker',
  name: 'CI pipeline',
  steps: [
    Step({
        name: 'restore cache',
        commands: ['restore-cache'],
        environment: cacheEnv,
        volumes: cacheVolumes,
        failure: 'ignore',
        when: {
            branch: {
                exclude: ['master']
            }
        }
    }),
    Step({
        name: 'check',
        commands: [
            'blaze run --target lint --all'
        ]
    }),
    Step({
        name: 'build',
        commands: [
            'blaze run cli:build-release',
            'blaze run --projects website,downloads --target build',
        ],
        volumes: dockerVolumes,
        when: {
            branch: ['master']
        }
    }),
    Step({
        name: 'test',
        commands: [
            'blaze run --parallelism None tests:run-all'
        ],
        environment: dockerCredentials,
        volumes: dockerVolumes,
        when: {
            branch: ['master']
        }
    }),
    Step({
        name: 'create cache',
        environment: cacheEnv,
        commands: ['create-cache'],
        volumes: cacheVolumes,
        failure: 'ignore',
        when: {
            branch: {
                exclude: ['master']
            }
        }
    })
  ],
  volumes: [
    {
      name: 'cache',
      host: {
        path: '/var/lib/cache',
      },
    },
    {
      name: 'docker-socket',
      host: {
        path: '/run/user/1002/docker.sock',
      },
    },
  ],
  trigger: {
    event: ['push', 'custom']
  },
  image_pull_secrets: ['DOCKER_REGISTRY_AUTHENTICATION_JSON'],
};

local publish = {
    kind: 'pipeline',
    type: 'docker',
    name: 'Publish pipeline',
    steps: [
        Step({
            name: 'deploy binaries',
            commands: [
                'blaze run --parallelism None cli:deploy'
            ],
            environment: dockerCredentials,
            volumes: [
                {
                    name: 'builds',
                    path: '/var/lib/blaze/builds'
                }
            ] + dockerVolumes
        }),
        Step({
            name: 'push tags',
            commands: [
                'blaze run ci:push-tags'
            ],
            volumes: [
                {
                    name: 'ssh',
                    path: '/root/.ssh'
                }
            ]
        }),
        Step({
            name: 'publish packages',
            commands: [
                'blaze run --all --target publish'
            ],
            environment: {
                CARGO_TOKEN: {
                    from_secret: 'CARGO_TOKEN'
                },
                NPM_TOKEN: {
                    from_secret: 'NPM_TOKEN'
                }
            },
            volumes: dockerVolumes
        })
    ],
    volumes: [
        {
            name: 'ssh',
            host: {
                path: '/var/lib/drone/.ssh',
            },
        },
        {
            name: 'builds',
            host: {
                path: '/var/lib/blaze/builds',
            },
        },
        {
            name: 'docker-socket',
            host: {
                path: '/run/user/1002/docker.sock',
            },
        },
    ],
    trigger: {
        event: ['promote']
    },
    image_pull_secrets: ['DOCKER_REGISTRY_AUTHENTICATION_JSON'],
};

[
    ci,
    publish
]