# `mod-horizon/`: the native mod on the official Horizon

`mod/` runs the Java runtime inside **Sunrise** (`sunrise-mod-api`, see [mod-plan.md](mod-plan.md)).
`mod-horizon/` is the same mod built on the **official Horizon**'s mod API (`horizon-mod-api` from
horizon-rust/horizon, merged there by horizon-rust/horizon#16941). It exists because the API is now
upstream and someone wanted Java mods in the official game; that side owns it (#310):

- **Gabbotampo's side owns this build**: its bridge code, its pin, its CI job and its in-game proofs.
  Sunrise-side bridge changes made in `mod/` are not ported here by the maintainers.
- **It never holds a runtime PR.** The `mod-horizon` CI job is `continue-on-error`, and the `gates`
  job's vendor check covers `mod/` only; when an interpreter change breaks this build, the fix is a
  follow-up here.
- **Its runtime copy is generated, never committed**: `python3 tools/vendor.py --mod mod-horizon`
  writes the same `crates/horizon-native-java/src` copy as `mod/src/runtime/` into
  `mod-horizon/src/runtime/` (ignored by git). The `mod-horizon` CI job runs it first; run it locally
  before a build. Plain `vendor.py` and `vendor.py --check` touch `mod/` only, so no runtime PR has to
  regenerate or commit anything for this build.

## The SDK and the lock

Like `mod/` on Sunrise, `mod-horizon/` names the mod API by the path of the pinned build's signed SDK,
`.horizon/sdk/api/crates/horizon-mod-api` (ignored by git): the official Horizon refuses every other
form under `E_SDK_SOURCE` (horizon-rust/horizon `docs/modding/src/reference/sdk/03-in-a-project.md`).
The `mod-horizon` CI job fetches it from the release `sdk-<pin>` and checks it as that project's
`01-release.md` says (Horizon's signature on `SHA256SUMS`, `sdk.json` naming the pin, each file's size
and SHA-256), then builds with the Rust release the SDK was compiled with (`toolchain.rust`).
Locally, the same folder is what `cargo horizon-mod check` fetches when run in `mod-horizon/` (per that
page), or the CI step can be run by hand.

`Cargo.lock` is committed and the job builds `--locked`. Every registry package it shares with the
game is at the version and checksum of Horizon's own `Cargo.lock` at the pin, so the mod brings no
second copy of them.

## What differs from `mod/`

The bridge code (`src/*.rs`) is `mod/`'s, re-derived from it whenever `mod/` changes, with only:

| `mod/` (Sunrise) | `mod-horizon/` (official Horizon) |
|---|---|
| `sunrise-mod.toml`, `[native] sunrise = …` | `horizon-mod.toml`, `[native] horizon = …` |
| `sunrise-mod-api` from `.sunrise/sdk/` | `horizon-mod-api` from `.horizon/sdk/` |
| `#[mod_api::sunrise_mod]` | `#[mod_api::horizon_mod]` |
| `HitHost` (damage hooks' level) | `LevelRandomHost` |
| `BlocksEvent::register`, `ItemsEvent::register` (checked per entry) | `event.blocks.push`, `event.items.push` (the game's ownership fence refuses at start) |
| Test fakes: `AttachmentHost::validate_attachment`, `ConfigFiles::remove`, `PlayerModesBackend` | Not in the official API; `player_is_spectator` is on `PlayersBackend` |

Everything else is the same code, `game_dispatch.rs` included: the bridge takes its state through the
same `StateGuard` and dispatch gate as `mod/`, so Java callbacks re-enter the same way on both games.
Comments that name the game say Horizon instead of Sunrise.

The pin is `0a875da91d` (Nightly 2026-11-20), the newest build with a published `sdk-<sha>` release
when this was written. At that pin `cargo check --all-targets`, clippy with `-D warnings` and
`cargo test --lib` pass.

## What is not done yet

- **No in-game proof.** The next step is the M0–M3b proof of [mod-plan.md](mod-plan.md) with the
  official launcher's `native-build mod-horizon --server-output …` at this pin.
