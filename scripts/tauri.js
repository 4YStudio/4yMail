import { spawn } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const tauriCli = require.resolve("@tauri-apps/cli/tauri.js");
const bundleTargets = new Set(["appimage", "deb", "rpm", "nsis", "dmg"]);
const [command, ...rest] = process.argv.slice(2);

let args = [command, ...rest].filter((arg) => arg !== undefined);

if (command?.startsWith("build:")) {
  const target = command.slice("build:".length).toLowerCase();

  if (!bundleTargets.has(target)) {
    console.error(
      `不支持的安装包格式：${target || "(空)"}。可选格式：${[...bundleTargets].join(", ")}`,
    );
    process.exit(1);
  }

  args = ["build", "--bundles", target, ...rest];
}

const child = spawn(process.execPath, [tauriCli, ...args], { stdio: "inherit" });

child.on("error", (error) => {
  console.error(`无法启动 Tauri CLI：${error.message}`);
  process.exitCode = 1;
});

child.on("exit", (code, signal) => {
  process.exitCode = signal ? 1 : (code ?? 1);
});
