import * as os from "os";
import * as vscode from "vscode";
import * as path from "path";
import type * as ra from "./lsp_ext";

import { Cargo, getRustcId, getSysroot } from "./toolchain";
import type { Ctx } from "./ctx";
import { prepareEnv } from "./run";
import { isCargoRunnableArgs, unwrapUndefinable } from "./util";

const debugOutput = vscode.window.createOutputChannel("Debug");
type DebugConfigProvider = (
    runnable: ra.Runnable,
    runnableArgs: ra.CargoRunnableArgs,
    executable: string,
    env: Record<string, string>,
    sourceFileMap?: Record<string, string>,
) => vscode.DebugConfiguration;

export async function makeDebugConfig(ctx: Ctx, runnable: ra.Runnable): Promise<void> {
    const scope = ctx.activeRustEditor?.document.uri;
    if (!scope) return;

    const debugConfig = await getDebugConfiguration(ctx, runnable);
    if (!debugConfig) return;

    const wsLaunchSection = vscode.workspace.getConfiguration("launch", scope);
    const configurations = wsLaunchSection.get<any[]>("configurations") || [];

    const index = configurations.findIndex((c) => c.name === debugConfig.name);
    if (index !== -1) {
        const answer = await vscode.window.showErrorMessage(
            `Launch configuration '${debugConfig.name}' already exists!`,
            "Cancel",
            "Update",
        );
        if (answer === "Cancel") return;

        configurations[index] = debugConfig;
    } else {
        configurations.push(debugConfig);
    }

    await wsLaunchSection.update("configurations", configurations);
}

export async function startDebugSession(ctx: Ctx, runnable: ra.Runnable): Promise<boolean> {
    let debugConfig: vscode.DebugConfiguration | undefined = undefined;
    let message = "";

    const wsLaunchSection = vscode.workspace.getConfiguration("launch");
    const configurations = wsLaunchSection.get<any[]>("configurations") || [];

    const index = configurations.findIndex((c) => c.name === runnable.label);
    if (-1 !== index) {
        debugConfig = configurations[index];
        message = " (from launch.json)";
        debugOutput.clear();
    } else {
        debugConfig = await getDebugConfiguration(ctx, runnable);
    }

    if (!debugConfig) return false;

    debugOutput.appendLine(`Launching debug configuration${message}:`);
    debugOutput.appendLine(JSON.stringify(debugConfig, null, 2));
    return vscode.debug.startDebugging(undefined, debugConfig);
}

function createCommandLink(extensionId: string): string {
    // do not remove the second quotes inside
    // encodeURIComponent or it won't work
    return `extension.open?${encodeURIComponent(`"${extensionId}"`)}`;
}

async function getDebugConfiguration(
    ctx: Ctx,
    runnable: ra.Runnable,
): Promise<vscode.DebugConfiguration | undefined> {
    if (!isCargoRunnableArgs(runnable.args)) {
        return;
    }
    const runnableArgs: ra.CargoRunnableArgs = runnable.args;

    const editor = ctx.activeRustEditor;
    if (!editor) return;

    const knownEngines: Record<string, DebugConfigProvider> = {
        "vadimcn.vscode-lldb": getCodeLldbDebugConfig,
        "ms-vscode.cpptools": getCCppDebugConfig,
        "webfreak.debug": getNativeDebugConfig,
    };
    const debugOptions = ctx.config.debug;

    let debugEngine = null;
    if (debugOptions.engine === "auto") {
        for (var engineId in knownEngines) {
            debugEngine = vscode.extensions.getExtension(engineId);
            if (debugEngine) break;
        }
    } else if (debugOptions.engine) {
        debugEngine = vscode.extensions.getExtension(debugOptions.engine);
    }

    if (!debugEngine) {
        const commandCCpp: string = createCommandLink("ms-vscode.cpptools");
        const commandCodeLLDB: string = createCommandLink("vadimcn.vscode-lldb");
        const commandNativeDebug: string = createCommandLink("webfreak.debug");

        await vscode.window.showErrorMessage(
            `Install [CodeLLDB](command:${commandCodeLLDB} "Open CodeLLDB")` +
                `, [C/C++](command:${commandCCpp} "Open C/C++") ` +
                `or [Native Debug](command:${commandNativeDebug} "Open Native Debug") for debugging.`,
        );
        return;
    }

    debugOutput.clear();
    if (ctx.config.debug.openDebugPane) {
        debugOutput.show(true);
    }
    // folder exists or RA is not active.
    // eslint-disable-next-line @typescript-eslint/no-unnecessary-type-assertion
    const workspaceFolders = vscode.workspace.workspaceFolders!;
    const isMultiFolderWorkspace = workspaceFolders.length > 1;
    const firstWorkspace = workspaceFolders[0];
    const maybeWorkspace =
        !isMultiFolderWorkspace || !runnableArgs.workspaceRoot
            ? firstWorkspace
            : workspaceFolders.find((w) => runnableArgs.workspaceRoot?.includes(w.uri.fsPath)) ||
              firstWorkspace;

    const workspace = unwrapUndefinable(maybeWorkspace);
    const wsFolder = path.normalize(workspace.uri.fsPath);
    const workspaceQualifier = isMultiFolderWorkspace ? `:${workspace.name}` : "";
    function simplifyPath(p: string): string {
        // see https://github.com/rust-lang/rust-analyzer/pull/5513#issuecomment-663458818 for why this is needed
        return path.normalize(p).replace(wsFolder, `\${workspaceFolder${workspaceQualifier}}`);
    }

    const env = prepareEnv(runnable.label, runnableArgs, ctx.config.runnablesExtraEnv);
    const executable = await getDebugExecutable(runnableArgs, env);
    let sourceFileMap = debugOptions.sourceFileMap;
    if (sourceFileMap === "auto") {
        // let's try to use the default toolchain
        const [commitHash, sysroot] = await Promise.all([
            getRustcId(wsFolder),
            getSysroot(wsFolder),
        ]);
        const rustlib = path.normalize(sysroot + "/lib/rustlib/src/rust");
        sourceFileMap = {};
        sourceFileMap[`/rustc/${commitHash}/`] = rustlib;
    }

    const provider = unwrapUndefinable(knownEngines[debugEngine.id]);
    const debugConfig = provider(runnable, runnableArgs, simplifyPath(executable), env);
    if (debugConfig.type in debugOptions.engineSettings) {
        const settingsMap = (debugOptions.engineSettings as any)[debugConfig.type];
        for (var key in settingsMap) {
            debugConfig[key] = settingsMap[key];
        }
    }

    if (debugConfig.name === "run binary") {
        // The LSP side: crates\verus-analyzer\src\main_loop\handlers.rs,
        // fn to_lsp_runnable(...) with RunnableKind::Bin
        debugConfig.name = `run ${path.basename(executable)}`;
    }

    const cwd = debugConfig["cwd"];
    if (cwd) {
        debugConfig["cwd"] = simplifyPath(cwd);
    }

    return debugConfig;
}

async function getDebugExecutable(
    runnableArgs: ra.CargoRunnableArgs,
    env: Record<string, string>,
): Promise<string> {
    const cargo = new Cargo(runnableArgs.workspaceRoot || ".", debugOutput, env);
    const executable = await cargo.executableFromArgs(runnableArgs.cargoArgs);

    // if we are here, there were no compilation errors.
    return executable;
}

function getCCppDebugConfig(
    runnable: ra.Runnable,
    runnableArgs: ra.CargoRunnableArgs,
    executable: string,
    env: Record<string, string>,
    sourceFileMap?: Record<string, string>,
): vscode.DebugConfiguration {
    return {
        type: os.platform() === "win32" ? "cppvsdbg" : "cppdbg",
        request: "launch",
        name: runnable.label,
        program: executable,
        args: runnableArgs.executableArgs,
        cwd: runnable.args.cwd || runnableArgs.workspaceRoot || ".",
        sourceFileMap,
        environment: Object.entries(env).map((entry) => ({
            name: entry[0],
            value: entry[1],
        })),
        // See https://github.com/rust-lang/rust-analyzer/issues/16901#issuecomment-2024486941
        osx: {
            MIMode: "lldb",
        },
    };
}

function getCodeLldbDebugConfig(
    runnable: ra.Runnable,
    runnableArgs: ra.CargoRunnableArgs,
    executable: string,
    env: Record<string, string>,
    sourceFileMap?: Record<string, string>,
): vscode.DebugConfiguration {
    return {
        type: "lldb",
        request: "launch",
        name: runnable.label,
        program: executable,
        args: runnableArgs.executableArgs,
        cwd: runnable.args.cwd || runnableArgs.workspaceRoot || ".",
        sourceMap: sourceFileMap,
        sourceLanguages: ["rust"],
        env,
    };
}

function getNativeDebugConfig(
    runnable: ra.Runnable,
    runnableArgs: ra.CargoRunnableArgs,
    executable: string,
    env: Record<string, string>,
    _sourceFileMap?: Record<string, string>,
): vscode.DebugConfiguration {
    return {
        type: "gdb",
        request: "launch",
        name: runnable.label,
        target: executable,
        // See https://github.com/WebFreak001/code-debug/issues/359
        arguments: quote(runnableArgs.executableArgs),
        cwd: runnable.args.cwd || runnableArgs.workspaceRoot || ".",
        env,
        valuesFormatting: "prettyPrinters",
    };
}

// Based on https://github.com/ljharb/shell-quote/blob/main/quote.js
function quote(xs: string[]) {
    return xs
        .map(function (s) {
            if (/["\s]/.test(s) && !/'/.test(s)) {
                return "'" + s.replace(/(['\\])/g, "\\$1") + "'";
            }
            if (/["'\s]/.test(s)) {
                return '"' + s.replace(/(["\\$`!])/g, "\\$1") + '"';
            }
            return s.replace(/([A-Za-z]:)?([#!"$&'()*,:;<=>?@[\\\]^`{|}])/g, "$1\\$2");
        })
        .join(" ");
}
