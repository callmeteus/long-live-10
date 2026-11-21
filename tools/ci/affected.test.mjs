import assert from "node:assert/strict";
import test from "node:test";

import { affectedChecks, RustPackages, withDependents } from "./affected.mjs";

test("a package change also selects its dependents", () => {
    assert.deepEqual(withDependents("catalog-schema"), [
        "agent",
        "agent-ffi",
        "catalog-schema",
        "compatibility-catalog",
        "compatibility-engine",
        "patch-core",
        "patch-helper",
        "patch-runtime",
        "release-tools",
        "ui-contract",
        "updater",
    ]);
});

test("an updater change selects release tools", () => {
    assert.deepEqual(withDependents("updater"), ["release-tools", "updater"]);
});

test("a UI-only change runs only Flutter checks", () => {
    const checks = affectedChecks(["packages/ui/lib/main.dart"]);

    assert.deepEqual(checks.rust, []);
    assert.equal(checks.hasRust, false);
    assert.equal(checks.ui, true);
    assert.equal(checks.catalog, false);
    assert.equal(checks.vm, false);
});

test("a UI contract change runs Rust and Flutter checks", () => {
    const checks = affectedChecks(["packages/ui-contract/src/lib.rs"]);

    assert.deepEqual(checks.rust, ["agent", "agent-ffi", "patch-helper", "ui-contract"]);
    assert.equal(checks.ui, true);
});

test("a catalog schema change runs catalog and Flutter checks", () => {
    const checks = affectedChecks(["packages/catalog-schema/src/lib.rs"]);

    assert.equal(checks.catalog, true);
    assert.equal(checks.ui, true);
    assert.ok(checks.rust.includes("compatibility-engine"));
});

test("a catalog data change runs only catalog validation", () => {
    const checks = affectedChecks(["catalogs/patch.json"]);

    assert.deepEqual(checks.rust, []);
    assert.equal(checks.catalog, true);
    assert.equal(checks.ui, false);
});

test("a patch-core change requests VM tests", () => {
    const checks = affectedChecks(["packages/patch-core/src/lib.rs"]);

    assert.equal(checks.vm, true);
    assert.ok(checks.rust.includes("patch-core"));
});

test("workspace manifest changes run every Rust package", () => {
    const checks = affectedChecks(["Cargo.toml"]);

    assert.equal(checks.rust.length, RustPackages.length);
    assert.equal(checks.ui, true);
    assert.equal(checks.vm, true);
});

test("workflow changes validate the full pipeline", () => {
    const checks = affectedChecks([".github/workflows/ci.yml"]);

    assert.equal(checks.ci, true);
    assert.equal(checks.rust.length, RustPackages.length);
    assert.equal(checks.ui, true);
    assert.equal(checks.catalog, true);
    assert.equal(checks.vm, true);
});

test("the package graph comes from Cargo, not a stale list", () => {
    // patch-helper and patch-runtime exist in Cargo.toml but were missing from
    // the old hand-maintained list. Deriving from `cargo metadata` keeps them.
    assert.ok(RustPackages.includes("patch-helper"));
    assert.ok(RustPackages.includes("patch-runtime"));
    assert.deepEqual(withDependents("patch-runtime"), [
        "agent",
        "agent-ffi",
        "patch-helper",
        "patch-runtime",
    ]);
});

test("documentation changes run nothing", () => {
    const checks = affectedChecks(["README.md", "docs/development/README.md"]);

    assert.deepEqual(checks.rust, []);
    assert.equal(checks.hasRust, false);
    assert.equal(checks.ui, false);
    assert.equal(checks.catalog, false);
    assert.equal(checks.vm, false);
});
