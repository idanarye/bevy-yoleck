# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]

## 0.16.0 - 2023-09-06
### Changed
- Upgrade Bevy to 0.12 (and bevy_egui to 0.23)

## 0.15.0 - 2023-10-15
### Changed
- Upgrade bevy_egui to 0.22
- [**BREAKING**] `YoleckUi` is a regular resource again. See the [migration
  guide](MIGRATION-GUIDES.md#migrating-to-yoleck-013).

## 0.14.1 - 2023-07-30
### Fixed
- `Vpeol2dCameraControl` reversing the Y axis when panning and zooming.

  Note that the implementation of this fix requires that the camera entity will
  have the `VpeolCameraState` component - without it `Vpeol2dCameraControl`
  will not work at all. I do not consider it a breaking change though, because:
  1. The documentation do imply that you need `VpeolCameraState`.
  2. `Vpeol3dCameraControl` was already requiring `VpeolCameraState`, and they
     are supposed to be equivalent.
  3. `vpeol_2d` is useless without `VpeolCameraState`, so I'm not expecting
     anyone to not be using it just so that they can use the camera controls.

  So having `Vpeol2dCameraControl` work without `VpeolCameraState` was an
  undocumented feature, and I'm going to release this as a bugfix version, not
  a minor version.

## 0.14.0 - 2023-07-18
### Changed
### Added
- `#[derive(Reflect)]` to several components. Requires the `bevy_reflect` flag.

- [**BREAKING**] Rename `YoleckRouteClickTo` to `VpeolRouteClickTo`.

## 0.13.0 - 2023-07-11
### Changed
- Upgrade Bevy to 0.11 (and bevy_egui to 0.21)
- [**BREAKING**] `YoleckUi` is now a non-`Send` resource. See the [migration
  guide](MIGRATION-GUIDES.md#migrating-to-yoleck-013).

## 0.12.0 - 2023-06-20
### Added
- An exclusive systems mechanism for edit systems that operate alone and can
  thus assume control over the input (e.g. mouse motion and clicks)
- Multiple selection with the Shift key, and `YoleckEdit` methods for editing
  multiple entities.

### Changed
- When creating a new entity that uses `Vpeol*dPosition`, an exclusive system
  will kick in to allow placing the entity with the mouse (instead of just
  placing it in the origin and letting the user drag it from there)

## 0.11.0 - 2023-04-06
### Added
- `YoleckBelongsToLevel` for deciding which entities to despawn when the level
  unloads/restarts. This is added automatically by Yoleck, but should also be
  added to entities created by the game.

## 0.10.0 - 2023-03-28
### Changed
- Model detection now raycasts against the meshes in addition to the AABB.

### Added
- Supported for 2D meshes in vpeol_2d.

## 0.9.0 - 2023-03-27
### Changed
- [**BREAKING**] This entire release is a huge breaking change. See the
  [migration guide](MIGRATION-GUIDES.md#migrating-to-yoleck-09).
- [**BREAKING**] Move to a new model, where each Yoleck entity can be composed
  of multiple `YoleckComponent`s.
- [**BREAKING**] The syntax of edit systems and populate systems has
  drastically changed.

### Added
- A mechanism for upgrading entity's data when their layout changes. See
  `YoleckEntityUpgradingPlugin`. This can be used to upgrade old games to use
  the new semantics introduced in this version.
- `vpeol_3d` is back in, without the dependencies and with better dragging.
- `yoleck::prelude`
- `yoleck::vpeol::prelude`

### Removed
- `vpeol_position_edit_adapter` and `VpeolTransform2dProjection`. Use `Vpeol2dPosition` instead.

## 0.8.0 - 2023-03-14
### Changed
- Add scroll area to editor window.

### Fixed
- Panic that happens sometimes when dragging an entity with children.

### Added
- `YoleckDirective::spawn_entity` for spawning entities from user code (e.g.
  for creating entity duplication buttons)

## 0.7.0 - 2023-03-09
### Changed
- Upgrade Bevy to 0.10 (and bevy_egui to 0.20)
- [**BREAKING**] `VpeolSystemLabel` becomes `VpeolSystemSet`, and uses Bevy's
  new system set semantics instead of the removed system label semantics. All
  sets of that system are configured to run during the `EditorActive` state.

### Added
- `Anchor` is taken into account when vpeol_2d checks clicks on text (previous to
  Bevy 0.10 it did not have an `Anchor` component, and just used top-left)

## 0.6.0 - 2023-03-06
### Changed
- [**BREAKING**] Vpeol names no longer container the "yoleck" prefix - so
  `YoleckVpeolXYZ` becomes `VpeolXYZ` and `yoleck_vpeol_xyz` becomes
  `vpeol_xyz`. Vpeol is enough to avoid conflicts.
- [**BREAKING**] `vpeol_2d` sends drag coordinates as `Vec3`, not `Vec2`.
- [**BREAKING**] `YoleckWillContainClickableChildren` is renamed to
  `VpeolWillContainClickableChildren` and is no longer reexported by
  `vpeol_2d`.

### Added
- [**BREAKING**] `VpeolCameraState` - must be placed on a camera in order for
  vpoel to work.
- [**BREAKING**] `Vpeol2dCameraControl` - must be placed on a camera in order
  for vpoel_2d to apply camera panning and scrolling.

## 0.5.0 - 2023-02-22
### Changed
- Update bevy-egui version to 0.19.

## 0.4.0 - 2022-11-14
### Changed
- Update Bevy version to 0.9 and bevy-egui version to 0.17.

### Added
- Ability to revert levels to their initial state:
  - `Wipe Level` button for ne` levels.
  - `REVERT` button for existing levels
  - This is important because otherwise the only ways to select a different
    level are to save the changes or restart the editor.

### Fixed
- Knobs remaining during playtest.

## 0.3.0 - 2022-08-18
### Changed
- Update Bevy version to 0.8 and bevy-egui version to 0.15.

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
