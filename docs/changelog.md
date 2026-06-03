# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).




## [Releases]

## v1.3.0 - June 3, 2026

### Added

- Added *link shortening* to the gateway. The web UI has been updated correspondingly. Try it out: [https://axle.axiommath.ai/check#r=7d70453f-813f-4d19-8de9-44793dafa835](https://axle.axiommath.ai/check#r=7d70453f-813f-4d19-8de9-44793dafa835)
- Added Claude web, desktop, and mobile support to the [`axiom-axle-mcp`](https://pypi.org/project/axiom-axle-mcp/) MCP server via a hosted endpoint at `https://mcp.axiommath.ai/mcp`. See the [Quick Start](https://axle.axiommath.ai/v1/docs/quickstart/#mcp-server) for details. Thanks to Andrew Sutherland for suggestions on setting up this hosted instance.
- Added three new fields to the `info` field of every response to identify the executor version your request was handled on: `_executor_commit_sha`, `_executor_docker_image_id`, and `_executor_artifact_sha256`.

### Changed

- Added a new option `theorems_only` (default `true`) to all tools that select over theorems/lemmas. These tools now have the ability to select over **all declaration kinds**: `theorem2lemma`, `theorem2sorry`, `simplify_theorems`, `repair_proofs`, `have2lemma`, `have2sorry`, `sorry2lemma`, `disprove`:

    - To use this feature, set `theorems_only` to `false`. For backwards compatibility (default), keep `theorems_only` set to `true`.
    - You can now sorry out any declaration body, simplify/repair any declaration containing a proof, and extract lemmas from any `sorry` locations and any `have` statement locations in any declarations, including definitions, opaques, instances, etc.
    - For `theorem2lemma` and `disprove`, the new setting is a no-op on non-theorem kinds.
    - Note that the value of `theorems_only` affects what the `names` and `indices` fields select over. When `theorems_only` is `false`, names and indices refer to **all** declarations, not just theorem kinds.

- Reworked `repair_proofs` (the first of several planned changes):

    - Added two new passes to `repair_proofs`: `remove_unknown_options`, which strips unknown options both at the command-level and within proofs/terms, and `enable_autoImplicit`, which restores the `autoImplicit` option at the beginning of a theorem if an unknown identifier error occurs in a theorem's type signature.
    - Added command-level re-elaboration to `repair_proofs`, allowing repairs to stack (for example, when applying terminal tactics reveals another error to fix).
    - `replace_unsafe_tactics` now warns the user when replacing `native_decide` with `decide +kernel` fails. The tactic location is now left untouched.
    - `apply_terminal_tactics` now warns when no terminal tactics could be successfully applied at a given location in `repair_proofs`.
    - Fixed a bug in `apply_terminal_tactics` allowing malformed proofs with metavariables to be counted as successes in `repair_proofs`.

- `verify_proof` now permits `partial def` and `opaque`. These checks were overly strict previously and do not raise soundness concerns.
- `merge` now deduplicates other declaration kinds: axioms, opaques, inductives, classes, structures, etc. Previously, only theorems and definitions were eligible for deduplication.
- `extract_decls` now names anonymous declarations (examples, anonymous instances) by their start position, line then column (e.g. `_example_12_0`), rather than a running counter (e.g. `_example_0`), so the placeholder is stable and remains unique across a file even when several share a line.
- Added the `merge_duplicates` (default `false`) option to `sorry2lemma`, which merges extracted lemmas that are duplicates (either with other lemmas, or to the existing top-level theorem/lemma from which they are extracted) by definitional equality into a single lemma with all callsites pointing at it. Existing behavior can be retained with the default setting `merge_duplicates=false`.


### Fixed

- Added faster, more graceful retries on certain classes of connection errors. Minor change.


## v1.2.1 - April 29, 2026

### Deprecated

- `extract_theorems` has been deprecated and will no longer be updated. Please use `extract_decls` instead, which supports all declaration kinds (def, theorem, lemma, abbrev, instance, structure, etc.).

### Changed

- The AXLE client now uses HTTP/2 by default. We don't expect any significant performance differences from this change, but feel free to file a bug report if this is not the case. Users may set the `http2` parameter to false in the client constructor to revert back to the original HTTP/1.1 settings.

### Added

- Added a new option `expand_scoped_notations` to the `normalize` tool, which delaborates scoped notations into their expanded forms. See the [`normalize` documentation page](https://axle.axiommath.ai/v1/docs/tools/normalize/#available-normalizations) for details.

### Fixed

- Fixed a bug in the executors causing requests to hang, occasionally resulting in abnormally high latencies.

## v1.2.0 - April 15, 2026

### Added

- Added two new fields in `extract_theorems` to be consistent with `extract_decls` (see below):
    - `kind`: always `theorem` for `extract_theorems`.
    - `declaration_messages`: same content as `theorem_messages`. `theorem_messages` is now deprecated and will be removed in a future update.
- Added `extract_decls`, an upgraded version of `extract_theorems` that extracts all declaration kinds.
    - New `kind` field in each document. Possible values: `theorem`, `def`, `abbrev`, `axiom`, `opaque`, `structure`, `class`, `class inductive`, `inductive`, `instance`, `example`, `unknown`
    - Note: Not all fields are meaningful for all declaration kinds (e.g., `proof_length`/`tactic_counts` only apply to theorems/lemmas with tactic proofs.)
    - This tool should be used instead of `extract_theorems` as it is a strict superset of functionality. `extract_theorems` will be deprecated in a future update.

### Fixed

- Added "Last Used" and "Requests (24h)" columns to the API key console page for better visibility into API key usage.


## v1.1.1 - April 8, 2026

### Changed

- [!] We are turning *on* the `autoImplicit` and turning *off* the `pp.unicode.fun` Lean options. AXLE will now automatically insert implicit variables when they are missing. **This is a significant behavioral change, check your code!** These settings are consistent with Lean's default. The previous options were remnants from internal use preferences.
- [!] **We have renamed `mathlib_linter` to `mathlib_options`**, which now sets `linter.mathlibStandardSet` to true, `autoImplicit` to false, `relaxedAutoImplicit` to false, and `pp.unicode.fun` to true. Use this toggle to enable the stricter defaults that Mathlib uses by convention.

### Added

- Added Lean 4.29.0 support.
- Added support for glob patterns in the `permitted_sorries` field for `verify_proof`. See the `verify_proof` documentation page under the `permitted_sorries` field for example use cases.

### Fixed

- Fixed a bug causing timeouts to be capped at 10 minutes. All requests now max out at 15 minutes (with documentation updated correspondingly).

## v1.1.0 - April 1, 2026

### Changed

- [!] Removed `document_messages` from the response of `extract_theorems` — to replicate old behavior, run the `content` field of the resulting documents through the `check` tool. This change significantly improves the speed of `extract_theorems`.
- [!] `includeEndPos` has been turned on for Lean messages. This changes the format from:
`-:4:38: error: Function expected at...`
to (when endPos is available):
`-:4:38-4:43: error: Function expected at...`
This change affects all tools with Lean messages.
- Significantly reworked the Lean executor pool backend.
    - Latency has been decreased by 50% in most cases. For longer requests, the new executors can be more than 5 times faster!
    - Previously, the first request to each environment required a ~10s warmup. This is no longer the case, and so requests will be more faithful to their Lean timeout limits (not including queueing / waiting for available slots).
    - Eliminates a security risk involving persistent Lean workers.
- Improved the Lean worker warm-up pipeline. Worker scale-up is also more aggressive than before. In the worst case, when all workers are completely occupied / offline, users should expect no more than a 2-3 minute delay before more worker capacity spins up.

### Fixed

- Removed redundant parsing resulting in occasional speedups in `repair_proofs`, `normalize`, etc. when content does not change.
- Pruned missing executors from the gateway registry. Fixes a bug with autoscaling improperly triggering.


## v1.0.2 - March 18, 2026

### Added

- Added explicit `okay` return value to `repair_proofs`

### Changed

- Improved error messages for unknown options in `simplify_theorems`, `repair_proofs`, `normalize`
- Improved error messages for `ignore_imports` error (with links to relevant docs)
- Improved the efficiency of `merge`, bringing down the time spent on large requests by 20-30%.


## v1.0.1 - March 11, 2026

### Added

- Added [Changelog](https://axle.axiommath.ai/v1/docs/changelog/) and [Troubleshooting](https://axle.axiommath.ai/v1/docs/troubleshooting/) to the documentation pages.

### Fixed

- Increased request limits and fixed a typo in the documentation. Users with an API key are now limited to 20 active requests, and anonymous users are limited to 10 active requests.
- Increased maximum timeout to 15 minutes (from 5 minutes).
- Environments are now sorted by prefix (alphabetically) and then by version number (more recent versions first)
- Fixed a bug with `disprove` failing to recognize implicit local variables. This bug was [found by Bulhwi Cha](https://leanprover.zulipchat.com/#narrow/channel/219941-Machine-Learning-for-Theorem-Proving/topic/Axiom.20Lean.20Engine/near/578064991) on Lean Zulip.


## v1.0.0 - March 4, 2026

### Added

- Initial release of AXLE Python client
- Async client (`AxleClient`) with all 14 API tools:
    - `verify_proof` - Verify proofs against formal statements
    - `check` - Check Lean code for errors
    - `extract_theorems` - Extract theorems with dependencies
    - `rename` - Rename declarations
    - `theorem2lemma` - Convert theorem/lemma keywords
    - `theorem2sorry` - Replace proofs with sorry
    - `merge` - Combine multiple Lean files
    - `simplify_theorems` - Simplify proofs
    - `repair_proofs` - Repair broken proofs
    - `have2lemma` - Extract have statements to lemmas
    - `have2sorry` - Replace have statements with sorry
    - `sorry2lemma` - Extract sorries and errors to lemmas
    - `disprove` - Attempt to disprove theorems
    - `normalize` - Standardize formatting
- CLI tool with commands for all tools
- Helper functions for string manipulation
- Configuration via environment variables
- Type hints and PEP 561 compliance
- Comprehensive documentation
