import { Schema, notEmptyString, strictObject } from './utils.js'

export const fileChangesMatcherSchema = {
    oneOf: [
        {
            ...notEmptyString,
            description: 'Pattern to use when matching files.'
        },
        strictObject({
            description: 'A pattern based file changes matcher.',
            properties: {
                pattern: {
                    ...notEmptyString,
                    description: 'Pattern to use when matching files.'
                },
                exclude: {
                    type: 'array',
                    items: notEmptyString,
                    description: 'An array of patterns to use when excluding matched files.',
                    default: []
                },
                root: {
                    ...notEmptyString,
                    description: 'Match files from a specific directory.'
                },
                behavior: {
                    enum: [
                        'Mixed',
                        'Timestamps',
                        'Hash'
                    ],
                    description: 'Defines the change detection strategy for this matcher.',
                    default: 'Mixed'
                }
            },
            required: ['pattern']
        })
    ]
} satisfies Schema