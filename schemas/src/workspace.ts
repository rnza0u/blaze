import { logLevelSchema } from './log-level.js'
import { parallelismSchema } from './parallelism.js'
import { projectSelectorSchema } from './project-selector.js'
import { Schema, notEmptyString, strictObject } from './utils.js'

const projectPathSchema = {
    description: 'Relative path from the workspace root to the project\'s root directory, where the project.json file is located (or any of its variants). The project configuration filename must not be appended.',
    examples: ['path/to/my/project'],
    ...notEmptyString 
}

export const workspaceSchema: Schema = {
    $schema: 'https://json-schema.org/draft/2020-12/schema',
    $id: 'https://blaze-monorepo.dev/schemas/workspace',
    properties: {
        name: {
            type: 'string',
            pattern: '^[a-zA-Z0-9\\-_]+$',
            description: 'The workspace name.'
        },
        projects: strictObject({
            description: 'A mapping composed of project names pointing to their locations.',
            patternProperties: {
                '^[a-zA-Z0-9\\-_/]+$': {
                    oneOf: [
                        projectPathSchema,
                        strictObject({
                            description: 'An object describing the project.',
                            properties: {
                                path: projectPathSchema,
                                tags: {
                                    type: 'array',
                                    items: notEmptyString,
                                    uniqueItems: true,
                                    description: 'A list of tags for this project. Useful for when selecting projects by tag.'
                                },
                                description: {
                                    ...notEmptyString,
                                    description: 'A description for this project. Useful for when describing the workspace.'
                                }
                            }
                        })
                    ]
                }
            }
        }),
        settings: strictObject({
            description: 'Global settings for the workspace',
            properties: {
                defaultSelector: {
                    description: 'A default project selector to be used when no selection option is used.',
                    ...projectSelectorSchema
                },
                selectors: strictObject({
                    default: {},
                    description: 'A mapping of re-usable named project selectors.',
                    patternProperties: {
                        '^[a-zA-Z0-9\\-_/]+$': projectSelectorSchema
                    }
                }),
                parallelism: {
                    ...parallelismSchema,
                    description: 'The default parallelism level to use when executing tasks (for commands such as `run`, `spawn`...)'
                },
                logLevel: {
                    ...logLevelSchema,
                    description: 'Default log level when running commands on this workspace.'
                },
                resolutionParallelism: {
                    ...parallelismSchema,
                    description: 'The default parallelism level to use when resolving executors.'
                }
            }
        })
    },
    required: ['projects', 'name']
}