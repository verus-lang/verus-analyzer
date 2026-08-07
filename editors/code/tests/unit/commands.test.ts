import * as assert from "node:assert/strict";

import {
    cargoNewArgs,
    determineNewProjectOpenAction,
    validateNewProjectName,
} from "../../src/new_project";
import { proofBlockStartLines } from "../../src/commands";
import type { Context } from ".";

export async function getTests(ctx: Context) {
    await ctx.suite("New project command", (suite) => {
        suite.addTest("rejects empty project name", async () => {
            assert.equal(validateNewProjectName("", []), "Project name cannot be empty.");
            assert.equal(validateNewProjectName("   ", []), "Project name cannot be empty.");
        });

        suite.addTest("rejects dot project names", async () => {
            assert.equal(validateNewProjectName(".", []), "Project name cannot be '.' or '..'.");
            assert.equal(validateNewProjectName("..", []), "Project name cannot be '.' or '..'.");
        });

        suite.addTest("rejects path separators", async () => {
            assert.equal(
                validateNewProjectName("foo/bar", []),
                "Project name cannot contain '/' or '\\' characters.",
            );
            assert.equal(
                validateNewProjectName("foo\\bar", []),
                "Project name cannot contain '/' or '\\' characters.",
            );
        });

        suite.addTest("rejects invalid Cargo package name characters", async () => {
            assert.equal(
                validateNewProjectName("foo.bar", []),
                "Project name can contain only alphanumeric characters, '-' or '_'.",
            );
            assert.equal(
                validateNewProjectName("foo bar", []),
                "Project name can contain only alphanumeric characters, '-' or '_'.",
            );
            assert.equal(
                validateNewProjectName("foo+bar", []),
                "Project name can contain only alphanumeric characters, '-' or '_'.",
            );
        });

        suite.addTest("rejects existing child folder collisions", async () => {
            assert.equal(
                validateNewProjectName("demo", ["demo"]),
                "A file or folder with this name already exists.",
            );
        });

        suite.addTest("accepts a normal project name", async () => {
            assert.equal(validateNewProjectName("demo-project", []), undefined);
        });

        suite.addTest("resolves addToWorkspace fallback without workspace", async () => {
            assert.equal(determineNewProjectOpenAction("addToWorkspace", false), "open");
        });

        suite.addTest("keeps addToWorkspace when workspace exists", async () => {
            assert.equal(determineNewProjectOpenAction("addToWorkspace", true), "addToWorkspace");
        });

        suite.addTest("defaults to ask for unknown values", async () => {
            assert.equal(determineNewProjectOpenAction(undefined, true), "ask");
            assert.equal(determineNewProjectOpenAction("ask", true), "ask");
        });

        suite.addTest("builds binary cargo args", async () => {
            assert.deepEqual(cargoNewArgs("bin", "demo"), ["new", "--bin", "demo"]);
        });

        suite.addTest("builds library cargo args", async () => {
            assert.deepEqual(cargoNewArgs("lib", "demo"), ["new", "--lib", "demo"]);
        });
    });

    await ctx.suite("Proof block folding", (suite) => {
        suite.addTest("filters, orders, and deduplicates proof block ranges", async () => {
            const ranges = [
                { startLine: 2, endLine: 12, kind: "region", collapsedText: "proof_block" },
                { startLine: 5, endLine: 8, kind: "region", collapsedText: "proof_block" },
                { startLine: 2, endLine: 9, kind: "region", collapsedText: "proof_block" },
                { startLine: 20, endLine: 20, kind: "region", collapsedText: "proof_block" },
                { startLine: 30, endLine: 35, kind: "region" },
                { startLine: 40, endLine: 45, kind: "imports", collapsedText: "proof_block" },
            ];

            assert.deepEqual(proofBlockStartLines(ranges, true), [5, 2]);
            assert.deepEqual(proofBlockStartLines(ranges, false), [2, 5]);
        });
    });
}
