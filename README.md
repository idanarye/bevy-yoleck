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
  * One simple such plugin included behind feature flag - `vpeol_2d`.
* Playtest the levels inside the editor.

## Examples:

* WASM examples - you can't save the levels because it's WASM, but you can edit the levels run playtests:
  * https://idanarye.github.io/bevy-yoleck/demos/example2d
* Example game:
  * Download binaries from https://aeon-felis.itch.io/danger-doofus
  * See the code at https://github.com/idanarye/sidekick-jam-entry-danger-doofus
  * Run the exeutable with `--editor` to edit the game levels with Yoleck.

## File Format

Yoleck saves the levels in JSON files that have the `.yol` extension. A `.yol`
file's top level is a tuple (actually JSON array) of three values:

* File metadata - e.g. Yoleck version.
* Level data (placeholder - currently an empty object)
* List of entities.

Each entity is a tuple of two values:

* Entity metadata - e.g. its type.
* Entity data - that's the user defined struct

The reason tuples are used instead of objects is to ensure ordering - to
guarantee the metadata can be read before the data. This is important because
the metadata is needed to parse the data.

Yoleck generates another JSON file in the same directory as the `.yol` files
called `index.yoli`. The purpose of this file is to let the game know what
level are available to it (in WASM, for example, the asset server cannot look
at a directory's contents). The index file containes a tuple of two values:

* Index metadata - e.g. Yoleck version.
* List of objects, each contain a path to a level file relative to the index
  file.

## Versions

| bevy | bevy-yoleck | bevy-egui |
|------|-------------|-----------|
| 0.10 | 0.7         | 0.20      |
| 0.9  | 0.5, 0.6    | 0.19      |
| 0.9  | 0.4         | 0.17      |
| 0.8  | 0.3         | 0.15      |
| 0.7  | 0.1, 0.2    | 0.14      |

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
