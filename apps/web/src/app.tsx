import { createAnalysisBridge } from "./bridge";
import { createEditorModel } from "./editor";
import { createGraphModel } from "./graph";
import { createPanelModel } from "./panels";

export function renderApp(): string {
  const editor = createEditorModel();
  const graph = createGraphModel();
  const panels = createPanelModel();
  const bridge = createAnalysisBridge();

  return [
    "<main>",
    `  <h1>${editor.title}</h1>`,
    `  <p>${graph.summary}</p>`,
    `  <p>${panels.summary}</p>`,
    `  <p>${bridge.summary}</p>`,
    "</main>",
  ].join("\n");
}
