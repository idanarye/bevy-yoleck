use std::any::{Any, TypeId};
use std::hash::{BuildHasher, Hash};

use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::BoxedArc;
use crate::editor::YoleckPassedData;

#[doc(hidden)]
#[derive(Default, Resource)]
pub struct YoleckKnobsCache {
    by_key_hash: HashMap<u64, Vec<CachedKnob>>,
}

#[doc(hidden)]
#[derive(Component)]
pub struct YoleckKnobMarker;

struct CachedKnob {
    key: Box<dyn Send + Sync + Any>,
    entity: Entity,
    keep_alive: bool,
}

impl YoleckKnobsCache {
    pub fn access<'a, K>(&mut self, key: K, commands: &'a mut Commands) -> KnobFromCache<'a>
    where
        K: 'static + Send + Sync + Hash + Eq,
    {
        let entries = self
            .by_key_hash
            .entry(self.by_key_hash.hasher().hash_one(&key))
            .or_default();
        for entry in entries.iter_mut() {
            if let Some(cached_key) = entry.key.downcast_ref::<K>()
                && key == *cached_key
            {
                entry.keep_alive = true;
                return KnobFromCache {
                    cmd: commands.entity(entry.entity),
                    is_new: false,
                };
            }
        }
        let cmd = commands.spawn(YoleckKnobMarker);
        entries.push(CachedKnob {
            key: Box::new(key),
            entity: cmd.id(),
            keep_alive: true,
        });
        KnobFromCache { cmd, is_new: true }
    }

    pub fn clean_untouched(&mut self, mut clean_func: impl FnMut(Entity)) {
        self.by_key_hash.retain(|_, entries| {
            entries.retain_mut(|entry| {
                if entry.keep_alive {
                    entry.keep_alive = false;
                    true
                } else {
                    clean_func(entry.entity);
                    false
                }
            });
            !entries.is_empty()
        });
    }

    pub fn drain(&mut self) -> impl '_ + Iterator<Item = Entity> {
        self.by_key_hash
            .drain()
            .flat_map(|(_, entries)| entries.into_iter().map(|entry| entry.entity))
    }
}

pub struct KnobFromCache<'a> {
    pub cmd: EntityCommands<'a>,
    pub is_new: bool,
}

/// An handle for intearcing with a knob from an edit system.
pub struct YoleckKnobHandle<'a> {
    /// The command of the knob entity.
    pub cmd: EntityCommands<'a>,
    /// `true` if the knob entity is just created this frame.
    pub is_new: bool,
    passed: HashMap<TypeId, BoxedArc>,
}

impl YoleckKnobHandle<'_> {
    /// Get data sent to the knob from external systems (usually interaciton from the level
    /// editor)
    ///
    /// The data is sent using [a directive event](crate::YoleckDirective::pass_to_entity).
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;;
    /// # use bevy_yoleck::vpeol::YoleckKnobClick;
    /// # #[derive(Component)]
    /// # struct Example {
    /// #     num_clicks_on_knob: usize,
    /// # };
    /// fn edit_example_with_knob(mut edit: YoleckEdit<&mut Example>, mut knobs: YoleckKnobs) {
    ///     let Ok(mut example) = edit.single_mut() else { return };
    ///     let mut knob = knobs.knob("click-counting");
    ///     knob.cmd.insert((
    ///         // setup the knobs position and graphics
    ///     ));
    ///     if knob.get_passed_data::<YoleckKnobClick>().is_some() {
    ///         example.num_clicks_on_knob += 1;
    ///     }
    /// }
    /// ```
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        if let Some(dynamic) = self.passed.get(&TypeId::of::<T>()) {
            dynamic.downcast_ref()
        } else {
            None
        }
    }
}

#[derive(SystemParam)]
pub struct YoleckKnobs<'w, 's> {
    knobs_cache: ResMut<'w, YoleckKnobsCache>,
    commands: Commands<'w, 's>,
    passed_data: Res<'w, YoleckPassedData>,
}

impl YoleckKnobs<'_, '_> {
    pub fn knob<K>(&mut self, key: K) -> YoleckKnobHandle<'_>
    where
        K: 'static + Send + Sync + Hash + Eq,
    {
        let KnobFromCache { cmd, is_new } = self.knobs_cache.access(key, &mut self.commands);
        let passed = self
            .passed_data
            .0
            .get(&cmd.id())
            // TODO: find a way to do this with the borrow checker, without cloning
            .cloned()
            .unwrap_or_default();
        YoleckKnobHandle {
            cmd,
            is_new,
            passed,
        }
    }
}
