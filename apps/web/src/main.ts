import { renderApp } from "./app";

const root = globalThis.document?.getElementById("app");

if (root) {
  root.innerHTML = renderApp();
}
