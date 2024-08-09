import React from 'react'
import CodeBlock from '@theme/CodeBlock'
import { useState } from 'react'

export default function CodeAlternative({ alternatives }){
    const [active, setActive] = useState(0)
    return (
        <>
            <ul className="tabs">
                {
                    alternatives.map((alt, i) => (
                        <li key={i} onClick={() => setActive(i)} 
                            style={{ fontSize: '0.80rem'}}
                            className={[
                                'tabs__item', 
                                'margin-top--none',
                                ...(i === active ? ['tabs__item--active'] : [])
                            ].join(' ')}>
                            {alt.name}
                        </li>
                    ))
                }
            </ul>
            <CodeBlock {...alternatives[active].view} >
                {alternatives[active].code}
            </CodeBlock>
        </>
    )
}