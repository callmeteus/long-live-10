# Governance and funding

Long Live 10 is a community project that extends Windows 10 22H2 x64 with a versioned security catalog, a local patch engine, and a desktop interface. This document explains how decisions are made, how funding works, and which commitments do not change when money is involved.

## Principles

- Essential security fixes stay free. Diagnosis, catalog data, planning, apply, revert, and the fixes needed to keep a supported target secure are available to everyone under the repository license.
- Evidence comes before claims. A capability is supported only after the validation tier in the [support matrix](docs/support-matrix.md) passes.
- Testing happens in disposable virtual machines. No experimental patch is approved for a physical host.
- Changes stay reversible. Planning keeps a recovery path, and apply and revert remain explicit.
- The project does not bypass activation, licensing, or entitlement checks.

## Roles

| Role | Responsibility |
|------|----------------|
| Contributor | Opens issues and pull requests, adds tests and documentation, and follows the safety rules. |
| Maintainer | Reviews changes, merges code, manages releases and signing keys, and applies this governance document. |
| Security reporter | Reports a vulnerability through the private advisory flow and helps with reproduction and disclosure. |

Maintainers are contributors who have consistently delivered reviewed changes and shown good judgment about safety and scope. A new maintainer is added by lazy consensus of the current maintainers. A maintainer who becomes inactive for six months may be moved to emeritus status.

## Decisions

- Day-to-day changes use pull request review and lazy consensus. If no maintainer objects within seven days, a change may merge when it has the required tests and documentation.
- Architecture changes need an ADR in `docs/decisions/`.
- Roadmap changes go through the Stillmark CLI. `ROADMAP.md` is not edited by hand.
- Scope changes need an explicit note in the pull request. The [support matrix](docs/support-matrix.md) is updated in the same change.
- Security decisions stay private until a fix or mitigation is available, then follow the disclosure process below.
- A decision that cannot reach consensus is resolved by a maintainer vote. A simple majority wins, and the result is recorded in the pull request or ADR.

## Funding

Funding covers infrastructure, test hardware, signing and release operations, and maintainer time. Funding never changes the safety rules or the support levels.

### What stays free

- The source code and catalog data under the repository license.
- Essential security fixes and the guidance needed to apply them safely.
- The diagnostics, planning, apply, and revert paths described as supported in the support matrix.
- Security advisories and recovery documentation.

### Optional paid services

Optional services may be offered to fund the work:

- priority support for a specific deployment;
- help preparing a compatible Windows 10 22H2 image or test environment;
- custom compatibility testing for software or hardware combinations;
- hosted catalog or artifact mirrors with availability commitments;
- training and integration work for organizations.

These services buy time and support, not exclusive access to fixes. A paid service must not remove a free capability, delay a security fix for non-paying users, or hide catalog data behind a paywall.

### What the project does not do

- No paywalled security fixes.
- No sale of user data or telemetry.
- No advertising inside the application.
- No proprietary relicensing of contributed code.
- No accepting funding that requires bypassing licensing, activation, or entitlement checks.
- No accepting funding that requires withholding a fix from the public catalog.

### Transparency

When funding exists, maintainers publish a periodic summary of income and spending by category. Donations and sponsorships are disclosed when they could influence a technical decision. A maintainer with a financial interest in a change must say so and abstain from the final review.

The project does not promise a bug bounty. Credit is given in the release notes when a reporter agrees to it.

## Security reports

Do not open a public issue for a vulnerability in Long Live 10 itself. Use GitHub's private security advisory flow for the repository. Include the affected version, reproduction steps, and the impact you observed. Maintainers aim to acknowledge a report within seven days.

Reports about Microsoft products belong with Microsoft. The project can document a mitigation and publish a catalog entry, but it cannot replace a vendor fix.

## Changes to this document

Governance changes use the same pull request and lazy consensus process as code. A change to the free-fix commitment needs approval from every active maintainer and a fourteen-day comment period. The result is written into this file, not only into the pull request.
