import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Options } from "@wdio/types";

const configDir = path.dirname(fileURLToPath(import.meta.url));
const appBinaryPath =
  process.env.TINOTAX_DESKTOP_BIN ??
  path.resolve(process.cwd(), "../../target/debug/tinotax-desktop.exe");

// Isolate the test app's WebView2 storage (localStorage, cache) in a throwaway
// folder. Without this the e2e shares the real app's user-data dir, so every
// run wipes and pollutes the user's Recent projects. WebView2 honours this env.
const webviewDataDir = fs.mkdtempSync(path.join(os.tmpdir(), "tinotax-e2e-webview-"));
process.env.WEBVIEW2_USER_DATA_FOLDER = webviewDataDir;

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
  onComplete() {
    // Best-effort: the WebView2 process may still hold a lock on its data dir.
    try {
      fs.rmSync(webviewDataDir, { recursive: true, force: true });
    } catch {
      // ignore — a stray temp dir is harmless and gets cleaned by the OS.
    }
  },
};
