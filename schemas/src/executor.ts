import { fileChangesMatcherSchema } from './matchers.js'
import { Schema, notEmptyString, strictObject } from './utils.js'

const executorKindSchema = {
    enum: ['Rust', 'Node']
} satisfies Schema

const gitPlainAuthentication = strictObject({
    properties: {
        username: notEmptyString,
        password: notEmptyString
    }
})

const httpTransportProperties: Record<string, Schema> = {
    insecure: {
        description: 'Disable SSL/TLS certificate verification. Only for debugging purpose.',
        type: 'boolean',
        default: false
    },
    headers: {
        type: 'object',
        description: 'Extra HTTP headers for the request.',
        patternProperties: {
            '^.+$': notEmptyString
        },
        default: {}
    }
}

const sshTransportProperties: Record<string, Schema> = {
    insecure: {
        description: 'Disable SSH server public key verification. Only for debugging purpose.',
        type: 'boolean',
        default: false
    },
    fingerprints: {
        description: 'An array of trusted SSH server fingerprints to use when validating.',
        type: 'array', 
        items: {
            type: 'string',
            pattern: '^(MD5|SHA1|SHA256):.+$'
        }
    }
}

const gitOptionsProperties: Record<string, Schema> = {
    path: {
        ...notEmptyString,
        description: 'Path to the executor within the repository. Defaults to the repository root directory if not provided.'
    },
    branch: {
        ...notEmptyString,
        description: 'Checkout a specific branch.'
    },
    rev: {
        ...notEmptyString,
        description: 'Checkout using a specific revision string (typically a commit hash).'
    },
    tag: {
        ...notEmptyString,
        description: 'Checkout a specific tag.'
    },
    kind: {
        ...executorKindSchema,
        description: 'Specify executor type if Blaze cannot infer it.'
    },
    pull: {
        type: 'boolean',
        description: 'Always pull last changes from remote.',
        default: false
    }
}

const sshAuthentication = {
    oneOf: [
        strictObject({
            description: 'Username/password authentication.',
            properties: {
                username: notEmptyString,
                password: notEmptyString
            },
            required: ['password']
        }),
        strictObject({
            description: 'Private key authentication.',
            properties: {
                key: notEmptyString,
                passphrase: notEmptyString,
                username: notEmptyString
            },
            required: ['key']
        })
    ]
} as const satisfies Schema

export const executorSchema = {
    oneOf: [
        {
            ...notEmptyString,
            description: 'Executor URL.'
        },
        strictObject({
            description: 'Standard executor configuration.',
            properties: {
                url: {
                    type: 'string',
                    enum: [
                        'noop',
                        'commands',
                        'exec'
                    ].map(name => `std:${name}`)
                }
            },
            required: ['url']
        }),
        strictObject({
            description: 'Local filesystem executor configuration',
            properties: {
                url: {
                    type: 'string',
                    description: 'URL containing the path to the executor files.',
                    pattern: '^file://.+$'
                },
                rebuild: {
                    enum: ['Always', 'OnChanges'],
                    description: 'When should Blaze rebuild the executor from source.',
                    default: 'OnChanges'
                },
                kind: {
                    ...executorKindSchema,
                    description: 'Executor kind, if Blaze cannot infer it.'
                },
                watch: {
                    type: 'array',
                    description: 'What files should be watched for changes when rebuild strategy is set to "OnChanges"',
                    items: fileChangesMatcherSchema
                }
            },
            required: ['url']
        }),
        strictObject({
            properties: {
                url: {
                    description: 'Git repository HTTP URL',
                    type: 'string',
                    pattern: '^https?://.+$'
                },
                format: {
                    description: 'Tells Blaze that the HTTP resource is a Git repository.',
                    const: 'Git'
                },
                authentication: {
                    ...gitPlainAuthentication,
                    description: 'Authentication to use when cloning over HTTP.'
                },
                ...httpTransportProperties,
                ...gitOptionsProperties
            },
            required: ['url', 'format'],
        }),
        strictObject({
            properties: {
                url: {
                    type: 'string',
                    description: 'Git repository SSH URL',
                    pattern: '^ssh://.+$'
                },
                authentication: {
                    description: 'Authentication to use when connecting to the SSH server.',
                    ...sshAuthentication
                },
                ...gitOptionsProperties,
                ...sshTransportProperties
            }
        })
    ]
} as const satisfies Schema