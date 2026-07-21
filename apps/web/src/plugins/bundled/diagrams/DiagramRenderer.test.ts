import { describe, expect, it } from "vitest";

import { normalizeDiagramSource } from "./DiagramRenderer";

describe("normalizeDiagramSource", () => {
  it("keeps Mermaid source as the source of truth", () => {
    const source = "flowchart LR\nCore --> Node";

    expect(normalizeDiagramSource("mermaid", source)).toBe(source);
  });

  it("converts the bounded PlantUML sequence subset", () => {
    expect(
      normalizeDiagramSource(
        "plantuml",
        "@startuml\nparticipant Core\nparticipant Node\nCore -> Node: dispatch\n@enduml",
      ),
    ).toBe(
      "sequenceDiagram\nparticipant Core\nparticipant Node\nCore->>Node: dispatch",
    );
  });

  it.each([
    ["mermaid", "%%{init: {'theme': 'dark'}}%%\nflowchart LR\nA-->B"],
    ["plantuml", "@startuml\n!include remote.puml\n@enduml"],
    ["mermaid", "<script>alert(1)</script>"],
  ])("rejects disabled directives for %s", (language, source) => {
    expect(() => normalizeDiagramSource(language, source)).toThrow(
      "disabled directive",
    );
  });
});
