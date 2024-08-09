import { dependenciesSchema } from './dependencies.js'
import { executorSchema } from './executor.js'
import { targetNameFormat } from './names.js'
import { targetCacheSchema } from './target-cache.js'
import { Schema, notEmptyString, strictObject } from './utils.js'

export const projectSchema: Schema = {
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    $id: 'https://blaze-monorepo.dev/schemas/project',
    type: 'object',
    title: 'Project',
    description: 'A project configuration within a Blaze workspace',
    properties: {
        targets: strictObject({
            patternProperties: {
                [targetNameFormat]: strictObject({
                    properties: {
                        executor: {
                            ...executorSchema,
                            description: 'The executor to use for this target.',
                        },   
                        options: {
                            description: 'Options for the executor.'
                        },
                        cache: targetCacheSchema,
                        dependencies: {
                            ...dependenciesSchema,
                            description: 'Dependencies that should be met before executing this target.',
                            default: []
                        },
                        description: {
                            ...notEmptyString,
                            description: 'A description for this target. Useful for when describing the project.'
                        },
                        stateless: {
                            type: 'boolean',
                            description: 'Can the target run concurrently (in multiple Blaze processes) ?',
                            default: false
                        }
                    }
                })
            }
        })
    },
    required: ['targets']
}