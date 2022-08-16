# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Removed
- **REGRESSION**: Removed `vpeol_3d` and `example3d`. They were depending on
  crates that were slow to migrate to Bevy 0.8 (one of then has still not
  released its Bevy 0.8 version when this changelog entry was written). Since
  `vpeol_3d` was barely usable to begin with (the gizmo is not a good way to
  move objects around - we need proper dragging! - and `bevy_mod_pickling`
  required lots of hacks to play nice with Yoleck) it has been removed for now
  and will be re-added in the future with less dependencies and better
  interface.

### Fixed
- Use the correct transform when dragging child entities (#11)

### Added
- Knobs!

## 0.2.0 - 2022-06-09
### Added
- `YoleckVpeolSelectionCuePlugin` for adding a pulse effect to show the
  selected entity in the viewport.

## 0.1.1 - 2022-06-02
### Fixed
- `vpeol_3d`: Entities sometimes getting deselected when cursor leaves egui area.
- `vpeol_3d`: Freshly created entities getting selected in Yoleck but Gizmo is not shown.

## 0.1.0 - 2022-06-01
### Added
- Building `YoleckTypeHandler`s to define the entity types.
- Editing entity structs with egui.
- Populating entities with components based on entity structs.
- Editor file manager.
- Level loading from files.
- Level index loading.
- `vpeol_2d` and `vpeol_3d`.
