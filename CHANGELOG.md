# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
