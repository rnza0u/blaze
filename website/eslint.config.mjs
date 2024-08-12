// @ts-check

import web from 'eslint-config-rnz-web'
import main from 'eslint-config-rnz-main'

export default [
    ...main,
    ...web,
    {
        ignores: [
            '.docusaurus',
            'sidebars.js', 
            'docusaurus.config.js', 
            'babel.config.js'
        ]
    }
]