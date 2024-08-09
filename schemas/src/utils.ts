import { JsonSchemaDraft202012 } from '@hyperjump/json-schema/draft-2020-12'

export type Schema = JsonSchemaDraft202012

export const strictObject: <T extends Exclude<Schema, boolean>> (schema: T) => T & { type: 'object', additionalProperties: false } = schema => {
    return {
        ...schema,
        type: 'object',
        additionalProperties: false
    }
}

export const notEmptyString = {
    type: 'string',
    minLength: 1
}  as const satisfies Schema