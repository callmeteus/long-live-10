import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

/**
 * Reads the workspace package graph from Cargo itself.
 *
 * The old lists were hand-maintained and drifted: they omitted patch-helper,
 * patch-runtime and several real dependencies. Deriving them from `cargo
 * metadata` makes the CI selection follow the actual build graph instead.
 */
function loadWorkspaceGraph() {
    const output = execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
        encoding: "utf8",
    });
    const metadata = JSON.parse(output);
    const names = new Set(metadata.packages.map((entry) => entry.name));
    const dependencies = {};

    for (const entry of metadata.packages) {
        dependencies[entry.name] = entry.dependencies
            .filter((dependency) => dependency.kind === null && names.has(dependency.name))
            .map((dependency) => dependency.name)
            .sort();
    }

    return {
        packages: [...names].sort(),
        dependencies,
    };
}

const graph = loadWorkspaceGraph();

export const RustPackages = graph.packages;
const DirectDependencies = graph.dependencies;

const ReverseDependencies = new Map(RustPackages.map((name) => [name, []]));

for (const [name, dependencies] of Object.entries(DirectDependencies)) {
    for (const dependency of dependencies) {
        ReverseDependencies.get(dependency).push(name);
    }
}

/**
 * Returns the package and every package that depends on it, directly or not.
 *
 * @param {string} name Rust package name.
 * @returns {string[]} Affected package names.
 */
export function withDependents(name) {
    const affected = new Set([name]);
    const pending = [name];

    while (pending.length > 0) {
        const current = pending.pop();

        for (const dependent of ReverseDependencies.get(current) ?? []) {
            if (!affected.has(dependent)) {
                affected.add(dependent);
                pending.push(dependent);
            }
        }
    }

    return [...affected].sort();
}

/**
 * Maps changed repository files to the checks that need to run.
 *
 * @param {string[]} files Repository-relative paths with forward slashes.
 * @returns {{ rust: string[], hasRust: boolean, ui: boolean, catalog: boolean, vm: boolean, ci: boolean }}
 */
export function affectedChecks(files) {
    const changed = new Set();
    let allRust = false;
    let ci = false;

    for (const raw of files) {
        const file = raw.replaceAll("\\", "/").replace(/^\.\//, "");

        if (file === "Cargo.toml" || file === "Cargo.lock") {
            allRust = true;
            continue;
        }

        if (file.startsWith("catalogs/") || file === "package.json") {
            changed.add("catalog");
            continue;
        }

        if (file.startsWith(".github/workflows/") || file.startsWith("tools/ci/")) {
            ci = true;
            continue;
        }

        const match = /^packages\/([^/]+)\//.exec(file);

        if (!match) {
            continue;
        }

        const name = match[1];

        if (name === "ui") {
            changed.add("ui");
        } else
        if (RustPackages.includes(name)) {
            changed.add(name);
        }
    }

    const rust = new Set();

    for (const name of changed) {
        if (RustPackages.includes(name)) {
            for (const affected of withDependents(name)) {
                rust.add(affected);
            }
        }
    }

    if (allRust || ci) {
        for (const name of RustPackages) {
            rust.add(name);
        }
    }

    // A package that pulls in ui-contract or catalog-schema also needs the
    // Flutter and catalog checks. Resolve this from the graph instead of a
    // hand-written list of "UI-adjacent" packages.
    const touchesUi = (name) =>
        withDependents(name).includes("ui-contract");
    const ui = ci ||
        allRust ||
        changed.has("ui") ||
        [...changed].some(touchesUi) ||
        (allRust && RustPackages.some(touchesUi));
    const catalog = ci ||
        changed.has("catalog") ||
        [...changed].some((name) =>
            withDependents(name).some((affected) =>
                affected === "catalog-schema" || affected === "compatibility-catalog"
            )
        );
    const vm = ci ||
        allRust ||
        [...changed].some((name) =>
            withDependents(name).some((affected) =>
                affected === "vm-harness" || affected === "patch-core" || affected === "agent"
            )
        );

    return {
        rust: [...rust].sort(),
        hasRust: rust.size > 0,
        ui,
        catalog,
        vm,
        ci,
    };
}

function changedFiles(base, head) {
    const output = execFileSync("git", ["diff", "--name-only", `${base}...${head}`], {
        encoding: "utf8",
    });

    return output
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter(Boolean);
}

function main() {
    const base = process.argv[2] ?? "HEAD^";
    const head = process.argv[3] ?? "HEAD";
    const checks = affectedChecks(changedFiles(base, head));

    console.log(JSON.stringify(checks, null, 4));

    if (process.env.GITHUB_OUTPUT) {
        appendFileSync(
            process.env.GITHUB_OUTPUT,
            [
                `rust=${JSON.stringify(checks.rust)}`,
                `has_rust=${checks.hasRust}`,
                `ui=${checks.ui}`,
                `catalog=${checks.catalog}`,
                `vm=${checks.vm}`,
                `ci=${checks.ci}`,
                "",
            ].join("\n"),
        );
    }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
    main();
}
