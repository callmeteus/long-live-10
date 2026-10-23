import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

export const RustPackages = [
    "agent",
    "agent-ffi",
    "catalog-schema",
    "compatibility-catalog",
    "compatibility-engine",
    "patch-core",
    "release-tools",
    "ui-contract",
    "updater",
    "vm-harness",
];

const DirectDependencies = {
    "agent": ["ui-contract"],
    "agent-ffi": ["agent", "ui-contract"],
    "catalog-schema": [],
    "compatibility-catalog": ["catalog-schema"],
    "compatibility-engine": ["compatibility-catalog", "catalog-schema"],
    "patch-core": ["catalog-schema"],
    "release-tools": ["updater", "catalog-schema"],
    "ui-contract": ["catalog-schema", "compatibility-catalog"],
    "updater": ["catalog-schema"],
    "vm-harness": [],
};

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

    const ui = ci ||
        allRust ||
        changed.has("ui") ||
        changed.has("ui-contract") ||
        changed.has("catalog-schema") ||
        changed.has("compatibility-catalog") ||
        changed.has("agent") ||
        changed.has("agent-ffi");
    const catalog = ci ||
        changed.has("catalog") ||
        changed.has("catalog-schema") ||
        changed.has("compatibility-catalog");
    const vm = ci ||
        allRust ||
        changed.has("vm-harness") ||
        changed.has("patch-core") ||
        changed.has("agent");

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
