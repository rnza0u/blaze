import { projectNameFormat } from './names.js'
import { Schema, notEmptyString, strictObject } from './utils.js'

export const projectSelectorSchema = {
    oneOf: [
        {
            description: 'Select all projects in the workspace.',
            const: 'All'
        },
        {
            type: 'array',
            description: 'A list of project names to select.',
            items: {
                type: 'string',
                pattern: projectNameFormat
            },
            uniqueItems: true
        },
        strictObject({
            properties: {
                tags: {
                    type: 'array',
                    items: notEmptyString,
                    minItems: 1,
                    uniqueItems: true
                }
            }
        }),
        strictObject({
            properties: {
                include: {
                    type: 'array',
                    items: {
                        description: 'An inclusion pattern to be used when selecting projects. Any project which name matches the pattern will be included in the selection.',
                        ...notEmptyString
                    },
                    uniqueItems: true,
                    minItems: 1
                },
                exclude: {
                    type: 'array',
                    items: {
                        description: 'An exclusion pattern (regular expression) to be used when selecting projects. Any project matching the pattern will be excluded from the selection.',
                        ...notEmptyString
                    },
                    uniqueItems: true
                }
            },
            required: ['include']
        })
    ]
} as const satisfies Schema