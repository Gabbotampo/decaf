# `mod-horizon/`: the native mod on the official Horizon

`mod/` runs the Java runtime inside **Sunrise** (`sunrise-mod-api`, see [mod-plan.md](mod-plan.md)).
`mod-horizon/` is the same mod built on the **official Horizon**'s mod API (`horizon-mod-api` from
horizon-rust/horizon, merged there by horizon-rust/horizon#16941). It exists because the API is now
upstream and someone wanted Java mods in the official game; that side owns it (#310):

- **Gabbotampo's side owns this build**: its bridge code, its pin, its CI job and its in-game proofs.
  Sunrise-side bridge changes made in `mod/` are not ported here by the maintainers.
- **It never holds a runtime PR.** The `mod-horizon` CI job is `continue-on-error`; when an
  interpreter change breaks it, the fix is a follow-up here.
- **The runtime is generated, never copied by hand**: `tools/vendor.py` writes the same
  `crates/horizon-native-java/src` copy into `mod/src/runtime/` and `mod-horizon/src/runtime/`, and
  `vendor.py --check` checks both.

## What differs from `mod/`

Only the API's names and the pin. The bridge code (`src/*.rs`) is `mod/`'s, with:

| `mod/` (Sunrise) | `mod-horizon/` (official Horizon) |
|---|---|
| `sunrise-mod.toml`, `[native] sunrise = …` | `horizon-mod.toml`, `[native] horizon = …` |
| `sunrise-mod-api` from `https://github.com/sunrise-org/sunrise` | `horizon-mod-api` from `https://github.com/horizon-rust/horizon` |
| `#[mod_api::sunrise_mod]` | `#[mod_api::horizon_mod]` |

The pin is a commit of horizon-rust/horizon `main` after #16941 (the merge commit itself does not
build `horizon-mod-vocab`: a leftover `tools/native-hook-fragments/*.json` is refused, fixed on main by
`3851860c`). At that pin `cargo check --all-targets`, clippy with `-D warnings` and `cargo test --lib`
pass with the same code as `mod/`, so the hooks and host calls the bridge uses are the same in both
APIs today.

## What is not done yet

- **No in-game proof.** A native mod builds against the prebuilt SDK of its pinned commit, and the
  official project has published no Nightly (and so no `sdk-<sha>` release) since #16941 merged. When
  one appears, the pin moves to that Nightly's commit, `Cargo.lock` is regenerated with
  `horizon-launcher native-lock mod-horizon`, and the M0–M3b proof of [mod-plan.md](mod-plan.md) is
  run with `horizon-launcher native-build mod-horizon --server-output …`.
- **Registry lock.** `native-build` accepts the SDK git dependency only with a lock written by
  `native-lock`; this folder has no `Cargo.lock` until the pin is a published build.
