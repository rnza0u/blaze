import { fileChangesMatcherSchema } from './matchers.js'
import { notEmptyString, strictObject } from './utils.js'

export const targetCacheSchema = strictObject({
    description: 'A target cache configuration object.',
    properties: {
        invalidateWhen: strictObject({
            properties: {
                inputChanges: {
                    type: 'array',
                    description: 'An array of input file changes matchers. Cache will invalidated if any of these matchers detect some file changes.',
                    items: fileChangesMatcherSchema,
                    uniqueItems: true
                },
                outputChanges: {
                    type: 'array',
                    description: 'An array of output file changes matchers. Cache will invalidated if any of these matchers detect some file changes.',
                    items: fileChangesMatcherSchema,
                    uniqueItems: true
                },
                filesMissing: {
                    type: 'array',
                    description: 'An array of file paths. Cache will be invalidated when any of these files is missing.',
                    items: notEmptyString,
                    uniqueItems: true
                },
                commandFails: strictObject({
                    description: 'A command configuration object. Cache will be invalidated if the command fails.',
                    properties: {
                        program: {
                            description: 'The name of the program to launch.',
                            ...notEmptyString
                        },
                        arguments: {
                            type: 'array',
                            description: 'An list of arguments for the program.',
                            default: [],
                            items: notEmptyString
                        },
                        environment: {
                            type: 'object',
                            description: 'Custom environment variables for the process.',
                            default: {},
                            patternProperties: {
                                '^.+$': {
                                    type: 'string'
                                }
                            }
                        },
                        verbose: {
                            description: 'Should command output be displayed ?',
                            default: false,
                            type: 'boolean'
                        },
                        cwd: notEmptyString
                    },
                    required: ['program']
                }),
                expired: strictObject({
                    description: 'A TTL configuration object. Cache will be invalidated after the given duration.',
                    properties: {
                        unit: {
                            description: 'The time unit to use.',
                            enum: [
                                'Milliseconds',
                                'Seconds',
                                'Minutes',
                                'Hours',
                                'Days'
                            ]
                        },
                        amount: {
                            description: 'The cache TTL value to use.',
                            type: 'integer',
                            minimum: 1
                        }
                    },
                    required: ['unit', 'amount']
                })
            }
        })
    }
})