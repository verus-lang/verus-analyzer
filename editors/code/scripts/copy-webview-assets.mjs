import { copyFile, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const extensionRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const outputDir = join(extensionRoot, "out", "webview");
const assets = [
    ["node_modules/d3/dist/d3.min.js", "d3.min.js"],
    ["node_modules/@hpcc-js/wasm/dist/graphviz.umd.js", "graphviz.umd.js"],
    ["node_modules/d3-graphviz/build/d3-graphviz.min.js", "d3-graphviz.min.js"],
];

await mkdir(outputDir, { recursive: true });
await Promise.all(
    assets.map(([source, destination]) =>
        copyFile(join(extensionRoot, source), join(outputDir, destination)),
    ),
);
