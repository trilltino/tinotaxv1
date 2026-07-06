import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";

const cargoHome = process.env.CARGO_HOME ?? path.join(os.homedir(), ".cargo");
process.env.PATH = [path.join(cargoHome, "bin"), process.env.PATH ?? ""].join(path.delimiter);

const tauri = path.join(
  process.cwd(),
  "node_modules",
  ".bin",
  process.platform === "win32" ? "tauri.cmd" : "tauri",
);

const result = spawnSync(tauri, process.argv.slice(2), {
  env: process.env,
  shell: process.platform === "win32",
  stdio: "inherit",
});

if (result.error) {
  console.error(result.error.message);
}

process.exit(result.status ?? 1);
