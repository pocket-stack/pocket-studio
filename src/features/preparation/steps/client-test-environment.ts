import { builtinEnvironments } from "vitest/runtime";

// Render SFC client templates with the memory renderer while retaining Node
// timers and globals for the suite.
export default {
  ...builtinEnvironments.node,
  name: "node-client",
  viteEnvironment: "client",
};
