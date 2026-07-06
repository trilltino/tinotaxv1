import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Options } from "@wdio/types";

const configDir = path.dirname(fileURLToPath(import.meta.url));
const appBinaryPath =
  process.env.TINOTAX_DESKTOP_BIN ??
  path.resolve(process.cwd(), "../../target/debug/tinotax-desktop.exe");

export const config: Options.Testrunner = {
  runner: "local",
  specs: [path.join(configDir, "*.e2e.ts")],
  maxInstances: 1,
  logLevel: "warn",
  framework: "mocha",
  reporters: ["spec"],
  services: [
    [
      "tauri",
      {
        appBinaryPath,
        driverProvider: "embedded",
      },
    ],
  ],
  capabilities: [{}],
  mochaOpts: {
    timeout: 180000,
  },
};
