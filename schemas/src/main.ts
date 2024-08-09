import { registerSchema, validate } from '@hyperjump/json-schema/draft-2020-12'
import { projectSchema } from './project.js'
import { exit } from 'node:process'
import { bundle } from '@hyperjump/json-schema/bundle'
import { mkdir, writeFile } from 'node:fs/promises'
import { join } from 'node:path'
import { workspaceSchema } from './workspace.js'

const schemas = {
    'project': projectSchema,
    'workspace': workspaceSchema
}

async function build(): Promise<void> {

    await mkdir('schemas', { recursive: true })

    await Promise.all(Object.entries(schemas).map(async ([name, schema]) => {

        if (typeof schema !== 'object' || !schema.$id)
            throw Error('no $id for schema')

        registerSchema(schema, schema.$id)
        
        const output = await validate('https://json-schema.org/draft/2020-12/schema', schema)
        
        if (!output.valid)
            throw Error(`invalid JSON schema (${schema.$id})`)

        const bundled = await bundle(schema.$id)
        
        await writeFile(join('schemas', `${name}-schema.json`), JSON.stringify(bundled, null, 4))
    }))
}

build().catch(err => {
    console.error(err)
    exit(1)
})