[![Build Status](https://github.com/idanarye/bevy-yoleck/workflows/CI/badge.svg)](https://github.com/idanarye/bevy-yoleck/actions)
[![Latest Version](https://img.shields.io/crates/v/bevy-yoleck.svg)](https://crates.io/crates/bevy-yoleck)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://idanarye.github.io/bevy-yoleck/)

# Bevy YOLECK - Your Own Level Editor Creation Kit

Yoleck is a crate for having a game built with the Bevy game engine act as its
own level editor.

## Features

* Same executable can launch in either game mode or editor mode, depending on
  the plugins added to the app.
* Write systems that create entities based on serializable structs - use same
  systems for both loading the levels and visualizing them in the editor.
* Entity editing is done with egui widgets that edit these structs.
* Support for external plugins that offer more visual editing.
  * Two simple such plugins included behind feature flags - `vpeol_2d` and `vpeol_3d`.
* Playtest the levels inside the editor.

## Planned features

* Better file management UI: support drag&drop reordering, renaming, deletion.
* Editable level data.
* Support non-linear level structure (e.g. graphs of which levels lead to where)
* Multiple entity selection and editing.
* Helpers for overlay editing (e.g. - handles the editor can interact with)
* Better 3D editing (the current
  [`bevy_mod_picking`](https://github.com/aevyrie/bevy_mod_picking) and
  [`bevy_transform_gizmo`](https://github.com/ForesightMiningSoftwareCorporation/bevy_transform_gizmo)
  integration feels clunky and cumbersome)

## Examples:

* WASM examples - you can't save the levels because it's WASM, but you can edit the levels run playtests:
  * https://idanarye.github.io/bevy-yoleck/demos/example2d
  * https://idanarye.github.io/bevy-yoleck/demos/example3d
* Example game:
  * Download binaries from https://aeon-felis.itch.io/danger-doofus
  * See the code at https://github.com/idanarye/sidekick-jam-entry-danger-doofus
  * Run the exeutable with `--editor` to edit the game levels with Yoleck.

## Versions

| bevy | bevy-yoleck |
|------|-------------|
| 0.7  |             |

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
