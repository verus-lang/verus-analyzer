import * as vscode from "vscode";
import * as os from "os";
import type { Config } from "./config";
import { type Env, log } from "./util";
import type { PersistentState } from "./persistent_state";
import { exec, execFile, spawnSync } from "child_process";
import fetch from "cross-fetch";
//import * as which from 'which';
//import which from "which";
import which = require("which");
import * as fs from 'fs';
import { promisify } from 'util';

export async function bootstrap(
    context: vscode.ExtensionContext,
    config: Config,
    state: PersistentState,
): Promise<string> {
    const path = await getServer(context, config, state);
    if (path) {
        log.info("Found Verus Analyzer server at: " + path);
    } else {
        throw new Error(
            "verus-analyzer Language Server is not available. " +
                "Please, ensure its [proper installation](https://github.com/verus-lang/verus-analyzer/).",
        );
    }

    if (!isValidExecutable(path, config.serverExtraEnv)) {
        throw new Error(
            `Failed to execute ${path} --version.` + config.serverPath
                ? `\`config.server.path\` or \`config.serverPath\` has been set explicitly.\
            Consider removing this config or making a valid server binary available at that path.`
                : "",
        );
    }

    return path;
}
async function getServer(
    context: vscode.ExtensionContext,
    config: Config,
    state: PersistentState,
): Promise<string | undefined> {
    const explicitPath = process.env["__RA_LSP_SERVER_DEBUG"] ?? config.serverPath;
    log.info("Explicit path to Verus Analyzer server binary: ", explicitPath);
    if (explicitPath) {
        if (explicitPath.startsWith("~/")) {
            return os.homedir() + explicitPath.slice("~".length);
        }
        return explicitPath;
    }
    if (config.package.releaseTag === null) {
        log.info("release tag is null");
        return "verus-analyzer";
    }

    const ext = process.platform === "win32" ? ".exe" : "";
    const bundled = vscode.Uri.joinPath(context.extensionUri, "server", `verus-analyzer${ext}`);
    const bundledExists = await vscode.workspace.fs.stat(bundled).then(
        () => true,
        () => false,
    );
    log.info("Bundled server exists: ", bundledExists);
    if (bundledExists) {
        let server = bundled;
        if (await isNixOs()) {
            server = await getNixOsServer(config, ext, state, bundled, server);
            await state.updateServerVersion(config.package.version);
        }
        return server.fsPath;
    }

    await vscode.window.showErrorMessage(
        "Unfortunately we don't ship verus-analyzer server binaries for your platform yet. " +
            "You need to manually clone the verus-analyzer repository and " +
            "run `cargo xtask install --server` to build the language server from sources. " +
            "If you feel that your platform should be supported, please create an issue " +
            "about that [here](https://github.com/verus-lang/verus-analyzer/issues) and we " +
            "will consider it.",
    );
    return undefined;
}

export async function getVerusVersion(
    verusPath: string | undefined,
): Promise<string> {
    log.info("Getting Verus version using Verus binary: ", verusPath);
    if (verusPath == undefined) {
        return "unknown";
    }
    const { stdout } = await execFileAsync(verusPath, [ "--version" ]);
    const version_regex : RegExp = /Version: (.*)/m;
    const matches = version_regex.exec(stdout);
    if (matches != null && matches.length > 1) {
        log.info("Found Verus version: ", matches[1]);
        if (matches[1] == undefined) {
            // TODO: This shouldn't be necessary.  Seems like a limitation of TypeScript's type system.
            return "unknown";
        }
        return matches[1];
    } else {
        log.info("Failed to find Verus version in: ", stdout);
        return "unknown";
    }
}

export async function getVerus(
    context: vscode.ExtensionContext,
    _config: Config,
): Promise<string|undefined> {
    const explicitPath: string | null | undefined = vscode.workspace.getConfiguration("verus-analyzer.verus").get("verusBinary");
    log.info("Explicit path to Verus binary: ", explicitPath);
    if (explicitPath != null) {
        if (explicitPath.startsWith("~/")) {
            const absPath = os.homedir() + explicitPath.slice("~".length);
            log.info("Absolute path to Verus binary:", absPath);
            return os.homedir() + explicitPath.slice("~".length);
        }
        return explicitPath;
    }
    const target_dir = vscode.Uri.joinPath(context.extensionUri, "verus");
    const ext = process.platform === "win32" ? ".exe" : "";
    const target_binary = vscode.Uri.joinPath(target_dir, `verus${ext}`);
    const target_dir_exists = await vscode.workspace.fs.stat(target_dir).then(
        () => true,
        () => false,
    );
    if (target_dir_exists) {
        log.info("Verus is already installed at: ", target_binary.fsPath, ".  No further work needed.")
        return target_binary.fsPath;
    } else {
        vscode.window.showInformationMessage("Attempting to determine the version of Verus's latest release...");
        const result = await fetch ('https://api.github.com/repos/verus-lang/verus/releases/latest',
            {
                method: 'get',
                headers: {
                    'Accept': 'application/vnd.github+json',
                    'X-GitHub-Api-Version': '2022-11-28',
                }
            }
        );

        if (result.status >= 400) {
            throw new Error("Bad response from server when attempting to fetch the latest Verus release.");
        }
          
        var platform = "";
        var release_dir = "";
        if (process.platform === "win32") { 
            platform = "win" 
            release_dir = "verus-x86-win";
        } else if (process.platform === "darwin" && process.arch === "x64") {
            platform = "x86-macos";
            release_dir = "verus-x86-macos";
        } else if (process.platform === "darwin") {
            platform = "arm64-macos";
            release_dir = "verus-arm64-macos";
        } else if (process.platform === "linux") { 
            platform = "linux" 
            release_dir = "verus-x86-linux";
        } else {
            await vscode.window.showErrorMessage(
                "Unfortunately we don't ship Verus binaries for your platform yet. " +
                    "You need to manually clone the verus repository and build it from sources. " +
                    "If you feel that your platform should be supported, please create an issue " +
                    "about that [here](https://github.com/verus-lang/verus/issues) and we " +
                    "will consider it.",
            );
            return;
        }
        log.info("Looking for a release for your platform, which we have identified as:", platform);
        log.info("We will save the downloaded Verus binaries in:", release_dir);

        const release_data = await result.json();
        for (const asset of release_data.assets) {
            log.info("Found release asset: ", asset.name);
            log.info("Index of your platform in the asset's name: ", asset.name.indexOf(platform));
            if (asset.name.indexOf(platform) >= 0) {
                vscode.window.showInformationMessage(`Attempting to download Verus's latest release (${asset.name})...`);
                // Download and store the release
                const url = asset.browser_download_url;
                log.info("Retrieving release from this URL:", url);
                const response = await fetch(url);
                const downloaded_release = vscode.Uri.joinPath(context.extensionUri, asset.name);
                await vscode.workspace.fs.writeFile(downloaded_release, new Uint8Array(await response.arrayBuffer()));
                // Unzip it
                const decompress = require('decompress');
                const t = vscode.Uri.joinPath(context.extensionUri, "unzipped"); //context.extensionUri.fsPath
                await decompress(downloaded_release.fsPath, t.fsPath);
                // Move it to a well-known location
                const src_dir = vscode.Uri.joinPath(t, release_dir); //context.extensionUri, release_dir);
                await vscode.workspace.fs.rename(src_dir, target_dir);
                vscode.window.showInformationMessage("Verus downloaded completed successfully.");
                vscode.window.showInformationMessage("Verus will run each time you save your file.");

                return target_binary.fsPath;
            }
        }
        await vscode.window.showErrorMessage(
            "We failed to find a Verus release asset matching your platform!" +
            `Consider manually installing it from [here](https://github.com/verus-lang/verus/) into: ${target_dir}`,
        );
        return;
    }
}


export async function findRustup(): Promise<{path: string|undefined}> {
    try {
        const resolvedPath = await which("rustup");
        log.info("Found rustup at: " + resolvedPath);
        return {path: resolvedPath };
    } catch(error: unknown) {
        log.warn("Caught an error while running `which(rustup)`: " + error);
        log.info("Attempting to find rustup in standard Cargo installation location...");

        // Try standard Cargo installation locations
        const ext = process.platform === "win32" ? ".exe" : "";
        const cargoHome = process.env.CARGO_HOME || (process.platform === "win32"
            ? `${process.env.USERPROFILE}\\.cargo`
            : `${os.homedir()}/.cargo`);
        const rustupPath = `${cargoHome}/bin/rustup${ext}`;

        try {
            const stats = await fs.promises.stat(rustupPath);
            if (stats.isFile()) {
                log.info("Found rustup at standard location: " + rustupPath);
                return { path: rustupPath };
            }
        } catch(statError: unknown) {
            log.warn(`Failed to find rustup at ${rustupPath}: ${statError}`);
        }

        return { path: undefined };
    }
}

const execFileAsync = promisify(execFile);

export async function validRustToolchain(): Promise<Boolean> {
    // TODO: Add a config flag for the expected toolchain version
    const TOOLCHAIN_FULL = 1;
    const TOOLCHAIN_MAJOR = 88;
    const TOOLCHAIN_MINOR = 0;

    const { path: rustup_executable } = await findRustup();
    if (!rustup_executable) {
        await vscode.window.showErrorMessage(
            "Failed to find rustup executable!",
        );
        return false;
    }
    try {
      const stats = await fs.promises.stat(rustup_executable);
      if(!stats.isFile()) {
        await vscode.window.showErrorMessage(
            rustup_executable + ' is not a valid file.'
        );
        return false;
      }
      const { stdout } = await execFileAsync(rustup_executable, [ "toolchain", "list" ]);
      const version_regex = /(\d+)\.(\d+)\.(\d+)-/ig;
      const toolchainVersions = [ ...stdout.matchAll(version_regex) ]
        .map(match => {
            if (match[1] == undefined || match[2] == undefined || match[3] == undefined) {
                log.warn("Undefined rustup version match groups: ", match)
                return { full: 0, major: 0, minor: 0 };
            } else {
                const full = parseInt(match[1], 10);
                const major = parseInt(match[2], 10);
                const minor = parseInt(match[3], 10);
                log.info(`Found a Rust toolchain version: ${full}.${major}.${minor}`);
                return { full, major, minor };
            }
        });
      if(toolchainVersions.find(({ full, major, minor }) =>
            full == TOOLCHAIN_FULL && major == TOOLCHAIN_MAJOR && minor == TOOLCHAIN_MINOR) == undefined) {
        const toolchain_str = `${TOOLCHAIN_FULL}.${TOOLCHAIN_MAJOR}.${TOOLCHAIN_MINOR}`;
        const cmd = `rustup toolchain install ${toolchain_str}`;
        await vscode.window.showErrorMessage(
            "Failed to find the Rust toolchain needed for Verus.  Try installing it by running: " + cmd
        );
        return false;
      } else {
        log.info("Found the expected rustup version");
        return true;
      }
    } catch(error: unknown) {
      const errorMsg = `Error invoking ${rustup_executable} toolchain list: ${error}`;
      console.error(errorMsg);
      return false//
    }
}

export function isValidExecutable(path: string, extraEnv: Env): boolean {
    log.debug("Checking availability of a binary at", path);

    const res = spawnSync(path, ["--version"], {
        encoding: "utf8",
        env: { ...process.env, ...extraEnv },
    });

    const printOutput = res.error ? log.warn : log.info;
    printOutput(path, "--version:", res);

    return res.status === 0;
}

async function getNixOsServer(
    config: Config,
    ext: string,
    state: PersistentState,
    bundled: vscode.Uri,
    server: vscode.Uri,
) {
    await vscode.workspace.fs.createDirectory(config.globalStorageUri).then();
    const dest = vscode.Uri.joinPath(config.globalStorageUri, `verus-analyzer${ext}`);
    let exists = await vscode.workspace.fs.stat(dest).then(
        () => true,
        () => false,
    );
    if (exists && config.package.version !== state.serverVersion) {
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
