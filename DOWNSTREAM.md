# DCode downstream architecture

DCode tracks `openai/codex` as an upstream-first distribution. The agent harness,
tool execution, protocol, rollout, and session machinery stay owned by upstream.

The downstream code is split into these boundaries:

- `codex-rs/dcode-product`: runtime product identity and defaults.
- `codex-rs/dcode-deepseek`: DeepSeek Responses, model discovery, API-key
  validation, and balance capabilities registered through provider factories.
- `codex-rs/core/src/vision.rs`: a bounded optional preprocessor for text-only
  primary models. It never rewrites existing history.
- `codex-rs/cli/src/bin/dcode.rs`: a thin launcher for the sibling upstream
  `codex` runtime, preserving upstream CLI behavior and tests.
- DCode-only workflows and installer scripts under `.github/workflows` and
  `scripts`.

Internal crate names remain `codex-*`. User-visible identity is selected at
runtime, so an upstream `codex` build retains its original behavior.

DCode also disables automatic OpenAI services at the product boundary. This
turns off analytics, feedback, curated plugin repository synchronization, and
cloud configuration loading by default without removing explicit provider
configuration or changing the behavior of the upstream `codex` binary.

## Syncing upstream

Keep DCode changes as a short commit stack on top of `upstream/main`. To inspect
the distance first:

```sh
scripts/sync-upstream.sh --check
```

With a clean worktree, apply the update with:

```sh
scripts/sync-upstream.sh --apply
scripts/check-downstream-boundary.py upstream/main
```

Resolve conflicts inside the files listed in
`.github/dcode-upstream-touchpoints.txt`; a conflict elsewhere indicates that a
downstream concern has leaked into the harness and should be extracted.

## Local storage

Development profiles already omit full debug symbols. Inspect disk use with
`scripts/dcode-storage.sh report`. Reclaim Cargo or Bazel artifacts explicitly
with `clean-cargo`, `clean-bazel`, or `clean-all`. These commands use the build
tools' own cleanup operations and never remove source files or DCode user data.
