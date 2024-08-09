import { projectSelectorSchema } from './project-selector.js'
import { Schema, notEmptyString, strictObject } from './utils.js'

export const dependenciesSchema = {
    type: 'array',
    items: {
        oneOf: [
            {
                description: 'The name of any target within the project',
                ...notEmptyString
            },
            strictObject({
                properties: {
                    target: notEmptyString,
                    projects: projectSelectorSchema,
                    optional: {
                        description: 'If set to true, target will be executed even if this dependency is not fullfilled.',
                        type: 'boolean'
                    },
                    cachePropagation: {
                        description: 'Defines how cache invalidations are propagated to this target\'s cache state.',
                        oneOf: [
                            {
                                const: 'Always',
                                description: 'Always propagate cache for this dependency.'
                            },
                            {
                                const: 'Never',
                                description: 'Never propagate cache for this dependency.'
                            }
                        ]
                    }
                },
                required: ['target']
            })
        ]
    },
    uniqueItems: true
} as const satisfies Schema