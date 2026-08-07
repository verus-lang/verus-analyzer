import * as vscode from "vscode";
import * as os from "os";
import type { Config } from "./config";
import { type Env, log, RUST_TOOLCHAIN_FILES, spawnAsync } from "./util";
import type { PersistentState } from "./persistent_state";
import { exec } from "child_process";
import { TextDecoder } from "node:util";
import { chmod } from "node:fs/promises";

export async function bootstrap(
    context: vscode.ExtensionContext,
    config: Config,
    state: PersistentState,
): Promise<string> {
    const path = await getServer(context, config, state);
    if (!path) {
        throw new Error(
            "verus-analyzer Language Server is not available. " +
                "Please ensure the Verus Analyzer extension or server is installed correctly.",
        );
    }

    log.info("Using server binary at", path);

    if (!(await isValidExecutable(path, config.serverExtraEnv))) {
        throw new Error(
            `Failed to execute ${path} --version.` +
                (config.serverPath
                    ? `\`config.server.path\` or \`config.serverPath\` has been set explicitly.\
            Consider removing this config or making a valid server binary available at that path.`
                    : ""),
        );
    }

    return path;
}
async function getServer(
    context: vscode.ExtensionContext,
    config: Config,
    state: PersistentState,
): Promise<string | undefined> {
    const packageJson: {
        version: string;
        releaseTag: string | null;
        enableProposedApi: boolean | undefined;
    } = context.extension.packageJSON;

    // check if the server path is configured explicitly
    const explicitPath = process.env["__RA_LSP_SERVER_DEBUG"] ?? config.serverPath;
    if (explicitPath) {
        if (explicitPath.startsWith("~/")) {
            return os.homedir() + explicitPath.slice("~".length);
        }
        return explicitPath;
    }

    let toolchainServerPath = undefined;
    if (vscode.workspace.workspaceFolders) {
        for (const workspaceFolder of vscode.workspace.workspaceFolders) {
            // otherwise check if there is a toolchain override for the current vscode workspace
            // and if the toolchain of this override has a verus-analyzer component
            // if so, use the verus-analyzer component
            // Check both rust-toolchain.toml and rust-toolchain files
            for (const toolchainFile of RUST_TOOLCHAIN_FILES) {
                const toolchainUri = vscode.Uri.joinPath(workspaceFolder.uri, toolchainFile);
                if (!(await hasToolchainFileWithRaDeclared(toolchainUri))) {
                    continue;
                }
                const res = await spawnAsync("rustup", ["which", "verus-analyzer"], {
                    env: { ...process.env },
                    cwd: workspaceFolder.uri.fsPath,
                });
                if (!res.error && res.status === 0) {
                    toolchainServerPath = await earliestToolchainPath(
                        toolchainServerPath,
                        res.stdout.trim(),
                        raVersionResolver,
                    );
                    break;
                }
            }
        }
    }
    if (toolchainServerPath) {
        return toolchainServerPath;
    }

    if (packageJson.releaseTag === null) return "verus-analyzer";

    // finally, use the bundled one
    const ext = process.platform === "win32" ? ".exe" : "";
    const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `verus-analyzer${ext}`);
    const bundledExists = await fileExists(bundled);
    if (bundledExists) {
        let server = bundled;
        if (await isNixOs()) {
            server = await getNixOsServer(
                context.globalStorageUri,
                packageJson.version,
                ext,
                state,
                bundled,
                server,
            );
            await state.updateServerVersion(packageJson.version);
        }
        return server.fsPath;
    }

    await vscode.window.showErrorMessage(
        "Unfortunately we don't ship binaries for your platform yet. " +
            "You need to manually clone the verus-analyzer repository and " +
            "run `cargo xtask install --server` to build the language server from sources. " +
            "If you feel that your platform should be supported, please create an issue " +
            "about that [here](https://github.com/verus-lang/verus-analyzer/issues) and we " +
            "will consider it.",
    );
    return undefined;
}

type VerusRelease = {
    assets: { name: string; browser_download_url: string }[];
};

export async function getVerus(
    context: vscode.ExtensionContext,
    config: Config,
): Promise<string | undefined> {
    const explicitPath = config.verusBinary;
    if (explicitPath) {
        const path = explicitPath.startsWith("~/")
            ? os.homedir() + explicitPath.slice("~".length)
            : explicitPath;
        if (!(await isValidExecutable(path, {}))) {
            throw new Error(`Configured Verus binary is not executable: ${path}`);
        }
        return path;
    }

    const executable = process.platform === "win32" ? "verus.exe" : "verus";
    const installDir = vscode.Uri.joinPath(context.globalStorageUri, "verus");
    const installedBinary = vscode.Uri.joinPath(installDir, executable);
    if (await fileExists(installedBinary)) {
        if (await isValidExecutable(installedBinary.fsPath, {})) {
            return installedBinary.fsPath;
        }
        await vscode.workspace.fs.delete(installDir, { recursive: true, useTrash: false });
    }

    const platform = verusReleasePlatform();
    if (!platform) {
        void vscode.window.showErrorMessage(
            `Verus does not publish a binary release for ${process.platform}/${process.arch}. ` +
                "Set `verus-analyzer.verus.binary` to a locally built Verus executable.",
        );
        return undefined;
    }

    return vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: "Installing Verus",
            cancellable: false,
        },
        async (progress) => {
            progress.report({ message: "Finding the latest release" });
            const releaseResponse = await fetch(
                "https://api.github.com/repos/verus-lang/verus/releases/latest",
                { headers: { Accept: "application/vnd.github+json" } },
            );
            if (!releaseResponse.ok) {
                throw new Error(
                    `GitHub returned ${releaseResponse.status} while locating Verus releases`,
                );
            }
            const release = (await releaseResponse.json()) as VerusRelease;
            const asset = release.assets.find((it) => it.name.includes(platform.assetMarker));
            if (!asset) {
                throw new Error(`No Verus release asset matched ${platform.assetMarker}`);
            }

            progress.report({ message: `Downloading ${asset.name}` });
            const assetResponse = await fetch(asset.browser_download_url);
            if (!assetResponse.ok) {
                throw new Error(`Failed to download ${asset.name}: HTTP ${assetResponse.status}`);
            }

            await vscode.workspace.fs.createDirectory(context.globalStorageUri);
            const archive = vscode.Uri.joinPath(context.globalStorageUri, asset.name);
            const staging = vscode.Uri.joinPath(context.globalStorageUri, "verus-staging");
            await vscode.workspace.fs.writeFile(
                archive,
                new Uint8Array(await assetResponse.arrayBuffer()),
            );
            await vscode.workspace.fs.delete(staging, { recursive: true, useTrash: false }).then(
                () => undefined,
                () => undefined,
            );
            await vscode.workspace.fs.createDirectory(staging);

            progress.report({ message: "Extracting Verus" });
            await extractArchive(archive.fsPath, staging.fsPath);
            const extracted = vscode.Uri.joinPath(staging, platform.releaseDirectory);
            if (!(await fileExists(extracted))) {
                throw new Error(
                    `The ${asset.name} archive did not contain ${platform.releaseDirectory}`,
                );
            }
            await vscode.workspace.fs.delete(installDir, { recursive: true, useTrash: false }).then(
                () => undefined,
                () => undefined,
            );
            await vscode.workspace.fs.rename(extracted, installDir, { overwrite: true });
            if (process.platform !== "win32") {
                await chmod(installedBinary.fsPath, 0o755);
            }
            await vscode.workspace.fs.delete(archive, { useTrash: false });
            await vscode.workspace.fs.delete(staging, { recursive: true, useTrash: false });

            if (!(await isValidExecutable(installedBinary.fsPath, {}))) {
                throw new Error(`Downloaded Verus binary failed its version check`);
            }
            return installedBinary.fsPath;
        },
    );
}

function verusReleasePlatform():
    | { assetMarker: string; releaseDirectory: string }
    | undefined {
    if (process.platform === "win32" && process.arch === "x64") {
        return { assetMarker: "x86-win", releaseDirectory: "verus-x86-win" };
    }
    if (process.platform === "darwin" && process.arch === "x64") {
        return { assetMarker: "x86-macos", releaseDirectory: "verus-x86-macos" };
    }
    if (process.platform === "darwin" && process.arch === "arm64") {
        return { assetMarker: "arm64-macos", releaseDirectory: "verus-arm64-macos" };
    }
    if (process.platform === "linux" && process.arch === "x64") {
        return { assetMarker: "x86-linux", releaseDirectory: "verus-x86-linux" };
    }
    return undefined;
}

async function extractArchive(archive: string, destination: string): Promise<void> {
    let attempts: [string, string[]][];
    if (process.platform === "win32") {
        attempts = [
            [
                "powershell.exe",
                [
                    "-NoProfile",
                    "-Command",
                    `Expand-Archive -LiteralPath '${archive.replaceAll("'", "''")}' -DestinationPath '${destination.replaceAll("'", "''")}' -Force`,
                ],
            ],
        ];
    } else if (archive.endsWith(".zip")) {
        attempts =
            process.platform === "darwin"
                ? [
                      ["ditto", ["-x", "-k", archive, destination]],
                      ["unzip", ["-q", archive, "-d", destination]],
                  ]
                : [
                      ["unzip", ["-q", archive, "-d", destination]],
                      ["tar", ["-xf", archive, "-C", destination]],
                  ];
    } else {
        attempts = [["tar", ["-xf", archive, "-C", destination]]];
    }

    for (const [command, args] of attempts) {
        const result = await spawnAsync(command, args);
        if (!result.error && result.status === 0) {
            return;
        }
        log.warn(`Failed to extract Verus with ${command}`, result);
    }
    throw new Error(
        `Could not extract ${archive}. Install an archive utility or configure verus.binary manually.`,
    );
}

// Given a path to a verus-analyzer executable, resolve its version and return it.
async function raVersionResolver(path: string): Promise<string | undefined> {
    const res = await spawnAsync(path, ["--version"]);
    if (!res.error && res.status === 0) {
        return res.stdout;
    } else {
        return undefined;
    }
}

// Given a path to two verus-analyzer executables, return the earliest one by date.
async function earliestToolchainPath(
    path0: string | undefined,
    path1: string,
    raVersionResolver: (path: string) => Promise<string | undefined>,
): Promise<string> {
    if (path0) {
        if (
            (await orderFromPath(path0, raVersionResolver)) <
            (await orderFromPath(path1, raVersionResolver))
        ) {
            return path0;
        } else {
            return path1;
        }
    } else {
        return path1;
    }
}

// Further to extracting a date for comparison, determine the order of a toolchain as follows:
//  Highest - nightly
//  Medium  - versioned
//  Lowest  - stable
// Example paths:
//  nightly   - /Users/myuser/.rustup/toolchains/nightly-2022-11-22-aarch64-apple-darwin/bin/verus-analyzer
//  versioned - /Users/myuser/.rustup/toolchains/1.72.1-aarch64-apple-darwin/bin/verus-analyzer
//  stable    - /Users/myuser/.rustup/toolchains/stable-aarch64-apple-darwin/bin/verus-analyzer
async function orderFromPath(
    path: string,
    raVersionResolver: (path: string) => Promise<string | undefined>,
): Promise<string> {
    const raVersion = await raVersionResolver(path);
    const raDate = raVersion?.match(/^verus-analyzer .*\(.* (\d{4}-\d{2}-\d{2})\)$/);
    if (raDate?.length === 2) {
        const precedence = path.includes("nightly-") ? "0" : "1";
        return "0-" + raDate[1] + "/" + precedence;
    } else {
        return "2";
    }
}

async function fileExists(uri: vscode.Uri) {
    return await vscode.workspace.fs.stat(uri).then(
        () => true,
        () => false,
    );
}

// Captures the elements of a `components` array. They are matched with `[^\]]` rather than `.`
// so that the array may be spread over several lines, which is just as valid TOML as keeping it
// on one, while still stopping at the end of the array.
const COMPONENTS_RE = /components\s*=\s*\[(?<components>[^\]]*)\]/;
// TOML strings come in both quote flavours.
const RA_COMPONENT_RE = /["']verus-analyzer["']/;

function declaresRaComponent(toolchainFileContents: string): boolean {
    const components = toolchainFileContents.match(COMPONENTS_RE)?.groups?.["components"];
    return components !== undefined && RA_COMPONENT_RE.test(components);
}

async function hasToolchainFileWithRaDeclared(uri: vscode.Uri): Promise<boolean> {
    try {
        const toolchainFileContents = new TextDecoder().decode(
            await vscode.workspace.fs.readFile(uri),
        );
        return declaresRaComponent(toolchainFileContents);
    } catch (_) {
        return false;
    }
}

export async function isValidExecutable(path: string, extraEnv: Env): Promise<boolean> {
    log.debug("Checking availability of a binary at", path);

    const newEnv = { ...process.env };
    for (const [k, v] of Object.entries(extraEnv)) {
        if (v) {
            newEnv[k] = v;
        } else if (k in newEnv) {
            delete newEnv[k];
        }
    }
    const res = await spawnAsync(path, ["--version"], {
        env: newEnv,
    });

    if (res.error) {
        log.warn(path, "--version:", res);
    } else {
        log.info(path, "--version:", res);
    }
    return res.status === 0;
}

async function getNixOsServer(
    globalStorageUri: vscode.Uri,
    version: string,
    ext: string,
    state: PersistentState,
    bundled: vscode.Uri,
    server: vscode.Uri,
) {
    await vscode.workspace.fs.createDirectory(globalStorageUri).then();
    const dest = vscode.Uri.joinPath(globalStorageUri, `verus-analyzer${ext}`);
    let exists = await vscode.workspace.fs.stat(dest).then(
        () => true,
        () => false,
    );
    if (exists && version !== state.serverVersion) {
        await vscode.workspace.fs.delete(dest);
        exists = false;
    }
    if (!exists) {
        await vscode.workspace.fs.copy(bundled, dest);
        await patchelf(dest);
    }
    server = dest;
    return server;
}

async function isNixOs(): Promise<boolean> {
    try {
        const contents = (
            await vscode.workspace.fs.readFile(vscode.Uri.file("/etc/os-release"))
        ).toString();
        const idString = contents.split("\n").find((a) => a.startsWith("ID=")) || "ID=linux";
        return idString.indexOf("nixos") !== -1;
    } catch {
        return false;
    }
}

async function patchelf(dest: vscode.Uri): Promise<void> {
    await vscode.window.withProgress(
        {
            location: vscode.ProgressLocation.Notification,
            title: "Patching verus-analyzer for NixOS",
        },
        async (progress, _) => {
            const expression = `
            {srcStr, pkgs ? import <nixpkgs> {}}:
                pkgs.stdenv.mkDerivation {
                    name = "verus-analyzer";
                    src = /. + srcStr;
                    phases = [ "installPhase" "fixupPhase" ];
                    installPhase = "cp $src $out";
                    fixupPhase = ''
                    chmod 755 $out
                    patchelf --set-interpreter "$(cat $NIX_CC/nix-support/dynamic-linker)" $out
                    '';
                }
            `;
            const origFile = vscode.Uri.file(dest.fsPath + "-orig");
            await vscode.workspace.fs.rename(dest, origFile, { overwrite: true });
            try {
                progress.report({ message: "Patching executable", increment: 20 });
                await new Promise((resolve, reject) => {
                    const handle = exec(
                        `nix-build -E - --argstr srcStr '${origFile.fsPath}' -o '${dest.fsPath}'`,
                        (err, stdout, stderr) => {
                            if (err != null) {
                                reject(Error(stderr));
                            } else {
                                resolve(stdout);
                            }
                        },
                    );
                    handle.stdin?.write(expression);
                    handle.stdin?.end();
                });
            } finally {
                await vscode.workspace.fs.delete(origFile);
            }
        },
    );
}

export const _private = {
    declaresRaComponent,
    earliestToolchainPath,
    orderFromPath,
};
