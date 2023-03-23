use std::any::{Any, TypeId};
use std::hash::{BuildHasher, Hash, Hasher};

use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::editor::YoleckPassedData;
use crate::BoxedArc;

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
    pub fn access<'w, 's, 'a, K>(
        &mut self,
        key: K,
        commands: &'a mut Commands<'w, 's>,
    ) -> KnobFromCache<'w, 's, 'a>
    where
        K: 'static + Send + Sync + Hash + Eq,
    {
        let mut hasher = self.by_key_hash.hasher().build_hasher();
        key.hash(&mut hasher);
        let entries = self.by_key_hash.entry(hasher.finish()).or_default();
        for entry in entries.iter_mut() {
            if let Some(cached_key) = entry.key.downcast_ref::<K>() {
                if key == *cached_key {
                    entry.keep_alive = true;
                    return KnobFromCache {
                        cmd: commands.entity(entry.entity),
                        is_new: false,
                    };
                }
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

pub struct KnobFromCache<'w, 's, 'a> {
    pub cmd: EntityCommands<'w, 's, 'a>,
    pub is_new: bool,
}

/// An handle for intearcing with a knob from an [edit system](YoleckEdit::edit).
pub struct YoleckKnobHandle<'w, 's, 'a> {
    /// The command of the knob entity.
    pub cmd: EntityCommands<'w, 's, 'a>,
    /// `true` if the knob entity is just created this frame.
    pub is_new: bool,
    passed: HashMap<TypeId, BoxedArc>,
}

impl YoleckKnobHandle<'_, '_, '_> {
    /// Get data sent to the knob from external systems (usually interaciton from the level
    /// editor)
    ///
    /// The data is sent using [a directive event](crate::YoleckDirective::pass_to_entity).
    ///
    /// ```no_run
    /// # use bevy::prelude::*;
    /// # use bevy_yoleck::prelude::*;;
    /// # #[derive(Component)]
    /// # struct Example {
    /// #     num_clicks_on_knob: usize,
    /// # };
    /// fn edit_example_with_knob(mut query: Query<&mut Example, With<YoleckEdit>>) {
    ///     let Ok(mut example) = query.get_single_mut() else { return };
    ///     let mut knob = knobs.knob("click-counting");
    ///     knob.insert((
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

impl<'w, 's> YoleckKnobs<'w, 's> {
    pub fn knob<'a, K>(&'a mut self, key: K) -> YoleckKnobHandle<'w, 's, 'a>
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
