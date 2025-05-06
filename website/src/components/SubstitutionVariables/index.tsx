import React from "react";
import CodeBlock from "@theme/CodeBlock";

type SubstitutionVariablesProps = Readonly<{
  vars: readonly Readonly<{
    path: string;
    description: string;
    example?: string;
  }>[];
}>;

export function SubstitutionVariables({ vars }: SubstitutionVariablesProps) {
  return (
    <table>
      <thead>
        <th scope="col">Path</th>
        <th scope="col">Description</th>
        <th scope="col">Example</th>
      </thead>
      <tbody>
        {vars.map((v) => (
          <tr>
            <td>
              <code>{v.path}</code>
            </td>
            <td>{v.description}</td>
            <td>
              {v.example &&
                (
                  <CodeBlock language="json">
                    {JSON.stringify(v.example, null, 2)}
                  </CodeBlock>
                )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
