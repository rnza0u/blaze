import React, { ComponentProps } from "react";
import CodeBlock from "@theme/CodeBlock";
import { useState } from "react";

type CodeAlternative = Readonly<{
  name: string;
  code: string;
  view?: ComponentProps<typeof CodeBlock>;
}>;

type CodeAlternativeProps = Readonly<{
  alternatives: readonly CodeAlternative[];
}>;

export function CodeAlternatives({ alternatives }: CodeAlternativeProps) {
  const [active, setActive] = useState(0);
  return (
    <>
      <ul className="tabs">
        {alternatives.map((alt, i) => (
          <li
            key={i}
            onClick={() => setActive(i)}
            style={{ fontSize: "0.80rem" }}
            className={[
              "tabs__item",
              "margin-top--none",
              ...(i === active ? ["tabs__item--active"] : []),
            ].join(" ")}
          >
            {alt.name}
          </li>
        ))}
      </ul>
      <CodeBlock {...alternatives[active].view}>
        {alternatives[active].code}
      </CodeBlock>
    </>
  );
}
