# Migrating to Yoleck 0.19

## `vpeol_read_click_on_entity`

Bevy 0.13 [split `WorldQuery` to `QueryData` and `FilterData`](https://bevyengine.org/learn/migration-guides/0-12-to-0-13/#split-worldquery-into-querydata-and-queryfilter) (though there is still a `WorldQuery` trait with some of that functionality). When you use `vpeol_read_click_on_entity`, the data passed to it is `QueryFilter`, not `QueryData` - which measn that if it's a component (which should usually be the case) you need `vpeol_read_click_on_entity::<Has<MyComponent>>` and not `vpeol_read_click_on_entity::<&MyComponent>` (which would have worked before)

# Migrating to Yoleck 0.17

## Loading levels

Instead of a `YoleckLoadingCommand` resource, level loading is now done via entities. This means that instead of loading a level like this:
```rust
fn load_level(
    mut yoleck_loading_command: ResMut<YoleckLoadingCommand>,
    asset_server: Res<AssetServer>,
) {
    *yoleck_loading_command = YoleckLoadingCommand::FromAsset(asset_server.load("levels/my-level.yol"));
}
```

You should do it like this:
```rust
fn load_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(YoleckLoadLevel(asset_server.load("levels/my-level.yol")));
}
```

Note that `YoleckLoadLevel` does not provide an equivalent for `YoleckLoadingCommand::FromData`. If you need to load a level from a value, put that value in `Assets<YoleckRawLevel>` first.

## Clearing levels

Instead of despawning all the entities marked with `YoleckBelongsToLevel`:

```rust
fn unload_level(
    query: Query<Entity, With<YoleckBelongsToLevel>>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
```

You should despawn the entities that represent the levels - the ones marked with `YoleckKeepLevel`:

```rust
fn unload_old_levels(
    query: Query<Entity, With<YoleckKeepLevel>>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
```

Yoleck will automatically despawn (with `despawn_recursive`) all the entities that belong to these levels.

Note that it is also possible, if needed, to just remove the `YoleckKeepLevel` component from these entities to despawn their entities without despawning the level entities themselves.

## Changes in `YoleckBelongsToLevel`

`YoleckBelongsToLevel` now has a `pub level: Entity` field that specifies which level the entity belongs to. When unloading a level (by despawning a `YoleckKeepLevel` entity, or removing the `YoleckKeepLevel` component from it), the entities that will be despawned are the ones who's `YoleckBelongsToLevel` points at that level.

As before, if you create a component from a system and want it to be despawned when switching a level or restarting/finishing a playtest in the editor, it still needs the `YoleckBelongsToLevel` component. Except now you have to provide a level entity for it. Where should the level entity come from? Two options:

* It can be attached to an existing level, so that its lifetime will be bound to it. This is useful for entities that need to exist in the level's space - when despawning the level, we don't want these entities to remain.

  The easiest way to achieve this is to use the `YoleckBelongsToLevel` of another component in that level. For example - say you have a treasure chest, and when the player shoots at it it opens up and a powerup pops from it for the player to pick up. Since the chest should already have a `YoleckBelongsToLevel` component, and since the system that spawns the powerup should already need to use some components of the chest entity, it should be easy to just clone the chest's `YoleckBelongsToLevel` and add it to the powerup spawning command.

* You can create a faux level and attach the entities to it. This is useful, for example, for a player character entity that can travel between levels. Just create a new entity with a `YoleckKeepLevel` component and add its `Entity` to the roaming entity inside a `YoleckBelongsToLevel` component.

  Note that you can freely set an existing `YoleckBelongsToLevel` to point to different levels. So it might make more sense to switch the player character entity to different level as it travels between them than to associate it to some faux level. Both options are available.

## Adding populate systems


`yoleck_populate_schedule_mut` is removed - this no longer works:

```rust
app.yoleck_populate_schedule_mut().add_systems(my_populate_system);
```

Instead, just add the system on the `YoleckSchedule::Populate` schedule:
```rust
app.add_systems(YoleckSchedule::Populate, my_populate_system);
```

`yoleck_populate_schedule_mut` made ergonomic sense in Bevy 0.10, but since starting Bevy 0.11 one has to always specify the schedule, it is no longer that ergonomic to have this helper method.

# Migrating to Yoleck 0.15

## Accessing the YoleckUi

Now that https://github.com/emilk/egui/pull/3233 got in to egui 0.23, and bevy_egui 0.22 was released with that new version of egui, `YoleckUi` can be made a regular resource again.

`YoleckUi` can no longer be accessed with `NonSend`/`NonSendMut`, and must be accessed with the regular `Res`/`ResMut`.

# Migrating to Yoleck 0.13

## Accessing the YoleckUi

`YoleckUi` is now a non-`Send` resource, which means it can no longer be accessed as a regular `Res`/`ResMut`. It must now be accessed as `NonSend`/`NonSendMut`.

Hopefully once https://github.com/emilk/egui/issues/3148 is fixed (and gets in to bevy_egui) this can be changed back.

# Migrating to Yoleck 0.9

## Importing

Most of the commonly used stuff can be imported from the new prelude module:

```rust
use bevy_yoleck::prelude::*;
```

## Entity type definition and registration

Previously entity types were declared as struct:

```rust
#[derive(Clone, PartialEq, Serialize, Deserialize)]
struct Foo {
    #[serde(default)]
    bar: Bar,
    #[serde(default)]
    baz: Baz,
}
```

And registered with:

```rust
app.add_yoleck_handler({
    YoleckTypeHandler::<Foo>::new("Foo")
        .populate_with(populate_foo)
        .edit_with(edit_foo)
});
```

Starting from 0.9, entities can be broken to multiple components:
```rust
#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
struct Bar {
    // ...
}

#[derive(Default, Clone, PartialEq, Serialize, Deserialize, Component, YoleckComponent)]
struct Baz {
    // ...
}
```

You can still create one big component per entity type, but if there are data
fields that are shared between different entity types it's better to split them
out so that they can be edited with the same edit systems.

Instead of registering type handlers, register entity types:

```rust
app.add_yoleck_entity_type({
    YoleckEntityType::new("Foo")
        .with::<Bar>()
        .with::<Baz>()
});
```

Unlike `YoleckTypeHandler`, that specifies the one data structure used by the
entity and all the edit and populate systems it'll have, `YoleckEntityType` can
specify multiple components and no systems. Systems are registered separately,
and are not bound to a single entity type:

```rust
app.add_yoleck_edit_system(edit_bar);
app.add_yoleck_edit_system(edit_baz);
app.yoleck_populate_schedule_mut().add_systems((
    populate_bar,
    populate_baz,
));
```

## Edit systems

In addition to the different method of registreation specified in the previous
section, the semantics of edit systems has also changed.

Previously, edit systems would use a closure:

```rust
fn edit_foo(mut edit: YoleckEdit<Foo>) {
    edit.edit(|ctx, data, ui| {
        // ...
    });
}
```

Now they use something that acts like a query:

```rust
fn edit_foo(mut ui: ResMut<YoleckUi>, mut edit: YoleckEdit<&mut Foo>) {
    let Ok(mut foo) = edit.get_single_mut() else { return };
    // ...
}
```

The differences:

* Instead of a closure, we use `get_single_mut` to get the single entity. If no
  entity is being edited, or if the edited entity does not match, we use
  `return` to skip the rest of the edit system.
  * In the future, when Yoleck will have multi-entity editing, `YoleckEdit`
    will have `iter` and `iter_mut` for edit systems that can edit multiple
    entities.
* Instead of getting the entity type directly as a generic parameter (`Foo`),
  `YoleckEdit` gets it like Bevy `Query`s would (`&mut Foo`). In fact,
  `YoleckEdit` can accept anything a Bevy query would accept, including filters
  as a second parameter.
* Instead of getting the UI handle via a closure argument, we get it as a
  resource in a separate `SystemParam` argument for the edit system function.

## Populate systems

In addition to the different method of registreation specified in an earlier
section, the semantics of populate systems has also changed.

Previously, populate systems would look like this:

```rust
fn populate_foo(mut populate: YoleckPopulate<Foo>) {
    populate.populate(|ctx, data, &mut cmd| {
        // ...
    });
}
```

Populate systems still use closures, but they look different:

```rust
fn populate_foo(mut populate: YoleckPopulate<&Foo>) {
    populate.populate(|ctx, &mut cmd, foo| {
        // ...
    });
}
```

The differences:

* Like `YoleckEdit`, `YoleckPopulate` also accepts query-like generic parameters.
* The command and data arguments to the closure switch places. Now the command
  is the second argument and the data is the third.
* The data argument is actually what a Bevy query with the same generic
  parameters as what the `YoleckPopulate` got would have yielded.

## Child entities

Previously, a populate system could freely use `cmd.despawn_descendants();`.
Now that there are multiple edit systems and their order is determined by a
scheduler, this should not be used, so instead populate systems should mark
child entities they create so that they can despawn them later (usually when
they replace them with freshly spawned ones):

```rust
fn populate_system(mut populate: YoleckPopulate<&MyComponent>, marking: YoleckMarking) {
    populate.populate(|_ctx, mut cmd, my_component| {
        marking.despawn_marked(&mut cmd);
        cmd.with_children(|commands| {
            let mut child = commands.spawn(marking.marker());
            child.insert((
                // relevant Bevy components
            ));
        });
    });
}
```

## Passed data

Previously, passed data would be accessed from the context argument of an edit system's closure:

## Knobs

Previously, knobs would be accessed from the context argument of an edit system's closure:

```rust
fn edit_foo(mut edit: YoleckEdit<Foo>, mut commands: Commands) {
    edit.edit(|ctx, data, ui| {
        let mut knob = ctx.knob(&mut commands, "knob-ident");
    });
}
```

Starting from 0.9 knobs are accessed with a new `SystemParam` named `YoleckKnobs`:

```rust
fn edit_foo(mut edit: YoleckEdit<&mut Foo>, mut knobs: YoleckKnobs) {
    let Ok(mut foo) = edit.get_single_mut() else { return };
    let mut knob = knobs.knob("knob-ident");
}
```

The actual usage of the knob handle is unchained.

Note that knobs are not associated to a specific edited entity (although they
do reset when the selection changes). This was also true before 0.9, but is
more visible now that they are not accessed from the edit closure's `ctx`.

## Position manipulation with vpeol_2d

* Instead of `vpeol_position_edit_adapter`, use `Vpeol2dPosition` as a Yoleck component.
* Don't set the translation by yourself - let vpeol_2d do it.
* If you need to also set rotation and scale, use `Vpeol2dRotatation` and
  `Vpeol2dScale`. vpeol_2d does not currently offer edit systems for them (it
  only takes them into account in the populate system), so you'll still have to
  write them yourself.
* `Vpeol2dPlugin` is split into two - `Vpeol2dPluginForEditor` and
  `Vpeol2dPluginForGame`. Use the appropriate one based on how the process
  started, just like you'd use the appropriate
  `YoleckPluginForEditor`/`YoleckPluginForGame`.
