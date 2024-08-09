import { Schema } from './utils.js'

export const logLevelSchema = {
    description: 'Log level when using commands.',
    oneOf: [
        {
            const: 'Error',
            description: 'Only errors will be logged.'
        },
        {
            const: 'Info',
            description: 'Informational messages and errors will be logged.'
        },
        {
            const: 'Warn',
            description: 'Information messages, errors and warnings will be logged.'
        },
        {
            const: 'Debug',
            description: 'Log everything including debugging messages.'
        }
    ],
    default: 'Warn'
} as const satisfies Schema