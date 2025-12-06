# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Drag and drop support for entity references: entities with UUID can now be dragged from the entity list and dropped onto `YoleckEntityRef` fields in the properties panel
- Entity type filtering is automatically applied when dropping entities onto entity reference fields with type constraints

## 0.36.0 - 2025-12-06
### Changed
- **Major UI/UX overhaul**: Redesigned editor interface with split-screen layout
  - Top panel: Level file management and playmode controls
  - Right panel: Selected entity properties editor
  - Bottom panel: Entity list and selection controls
  - Left panel: Entity type creation tools
- Improved editor ergonomics with better organized workspace instead of single cluttered panel

## 0.35.0 - 2025-12-05
### Changed
- Axis knobs are now always displayed for all 3 world axes (X, Y, Z) by default
- Axis knobs now have distinct colors (red for X, green for Y, blue for Z)
- Scene gizmo now displays axis labels (X, Y, Z) at the end of positive axes

### Removed
- [**BREAKING**] Removed `Vpeol3dThirdAxisWithKnob` component (no longer needed as all axes have knobs by default)

## 0.34.0 - 2025-12-04
### Changed
- `YoleckAutoEdit` derive macro now also implements `YoleckEntityRefAccessor`, eliminating the need for separate `YoleckEntityRefs` derive
- `add_yoleck_auto_edit` now registers both auto edit and entity ref edit systems automatically
- `YoleckEntityRef` fields are now automatically hidden from auto edit UI (rendered only by entity ref system)

### Removed
- Removed `YoleckEntityRefPlugin` (was empty/no-op)
- Removed `add_yoleck_entity_ref_edit` method (merged into `add_yoleck_auto_edit`)
- Removed `add_yoleck_full_edit` method (no longer needed)
- Removed `YoleckEntityRefs` derive macro (merged into `YoleckAutoEdit`)

## 0.33.0 - 2025-12-04
### Changed
- Added scene gizmo for camera orientation

## 0.32.0 - 2025-12-04
### Changed 
- Added automatic UI generation for components using reflection and attributes.
- Supported numeric, boolean, string, vector, color, enum, option, list, asset, and entity fields.
- Added EntityRef type with automatic UI, filtering, and runtime UUID resolution.
- Enabled entity linking with dropdown, viewport click, and drag-and-drop selection.

## 0.31.0 - 2025-12-04
### Changed 
- Make camera controls fps style 

## 0.30.0 - 2025-12-04
### Changed
- Update bevy_egui version to 0.38.

## 0.29.0 - 2025-10-03
### Changed
- Upgrade Bevy to 0.17
- Rename:
  - `YoleckSystemSet` -> `YoleckSystems`
  - `VpeolSystemSet` -> `VpeolSystems`

## 0.28.0 - 2025-08-05
### Changed
- Update bevy_egui version to 0.36.

## 0.27.0 - 2025-07-03
### Changed
- Update bevy_egui version to 0.35.

## 0.26.1 - 2025-06-04
### Fixed
- Don't fail when adding `VpeolRouteClickTo` to a non-existing child.

## 0.26.0 - 2025-04-26
### Changed
- Upgrade Bevy to 0.16
- [**BREAKING**] Rename `YoleckEdit`'s method `get_single` and `get_single_mut`
  to `single` and `single_mut` (to mirror a similar change in Bevy itself)
- Replace anyhow usage with `BevyError`.

## 0.25.0 - 2025-02-19
### Changed
- Update bevy_egui version to 0.33.

## 0.24.0 - 2025-01-09
### Changed
- Update bevy_egui version to 0.32.

## 0.23.0 - 2024-12-02
### Changed
- Update Bevy version to 0.15 and bevy_egui version to 0.31.

## 0.22.0 - 2024-07-06
### Changed
- Upgrade Bevy to 0.14 (and bevy_egui to 0.28)

## 0.21.0 - 2024-05-08
### Changed
- Update bevy_egui version to 0.27.

## 0.20.1 - 2024-04-01
### Fixed
- Enable bevy_egui's `render` feature, so that users won't need to load it
  explicitly and can use the one reexported from Yoleck (fixes
  https://github.com/idanarye/bevy-yoleck/issues/39)

## 0.20.0 - 2024-03-19
### Changed
- Update bevy_egui version to 0.26.

## 0.19.0 - 2024-02-27
### Changed
- Update Bevy version to 0.13 and bevy_egui version to 0.25.
- [**BREAKING**] Changed some API types to use Bevy's new math types. See the
  [migration guide](MIGRATION-GUIDES.md#migrating-to-yoleck-019).

## 0.18.0 - 2024-02-18
### Changed
- Upgrade bevy_egui to 0.24.

### Fixed
- [**BREAKING**] Typo - `Rotatation` -> `Rotation` in Vpeol.

## 0.17.1 - 2024-01-14
### Fixed
- Use a proper `OR` syntax for the dual license.

## 0.17.0 - 2023-11-25
### Removed
- [**BREAKING**] The `YoleckLoadingCommand` resource is removed, in favor of a
  `YoleckLoadLevel` component. See the [migration
  guide](MIGRATION-GUIDES.md#migrating-to-yoleck-017).
  - Note that unlike `YoleckLoadingCommand` that could load the level from
    either an asset or a value, `YoleckLoadLevel` can only load from an asset.
    If it is necessary to load a level from memory, add it to
    `ResMut<Assets<YoleckRawLevel>>` first and pass the handle to
    `YoleckLoadLevel`.
- [**BREAKING**] The `yoleck_populate_schedule_mut` method (which Yoleck was
  adding as an extension on Bevy's `App`) is removed in favor of just using
  `YoleckSchedule::Populate` directly.

### Added
- `YoleckEditableLevels` resource (accessible only from edit systems) that
  provides the list of level file names.
- Entity reference with `YoleckEntityUuid` and `YoleckUuidRegistry`.
  - Some picking helpers for handling entity references in the editor:
    `vpeol_read_click_on_entity`, `yoleck_map_entity_to_uuid` and
    `yoleck_exclusive_system_cancellable`.
- Load multiple levels with `YoleckLoadLevel` (which is a component, that can
  be placed on multiple entities)
- Unload levels by removing the `YoleckKeepLevel` component from the entity
  that was used to load the level - or by despawning that entity entirely.
- `YoleckSchedule::LevelLoaded` schedule for interfering with levels before
  populating their entities.
- `VpeolRepositionLevel` component.

### Change
- `YoleckBelongsToLevel` now points to a level entity.
- `YoleckDirective::spawn_entity` needs to know which level entity to create
  the component on.

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
- [**BREAKING**] `VpeolSystemLabel` becomes `VpeolSystems`, and uses Bevy's
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
