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

Unlike `YoleckTypeHandler`, that specifiy the one data structure used by the
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
