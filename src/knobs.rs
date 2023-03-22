use std::any::{Any, TypeId};
use std::hash::{BuildHasher, Hash, Hasher};

use bevy::ecs::system::{EntityCommands, SystemParam};
use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::BoxedArc;

#[doc(hidden)]
#[derive(Default, Resource)]
pub struct YoleckKnobsCache {
    by_key_hash: HashMap<u64, Vec<CachedKnob>>,
}

#[doc(hidden)]
#[derive(Component)]
pub struct YoleckKnobData {
    pub(crate) passed_data: HashMap<TypeId, BoxedArc>,
}

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
        let cmd = commands.spawn(YoleckKnobData {
            passed_data: Default::default(),
        });
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
    /// editor). See [`YoleckEdit::get_passed_data`].
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
    knobs_query: Query<'w, 's, &'static YoleckKnobData>,
}

impl<'w, 's> YoleckKnobs<'w, 's> {
    pub fn knob<'a, K>(&'a mut self, key: K) -> YoleckKnobHandle<'w, 's, 'a>
    where
        K: 'static + Send + Sync + Hash + Eq,
    {
        let KnobFromCache { cmd, is_new } = self.knobs_cache.access(key, &mut self.commands);
        let passed = self
            .knobs_query
            .get(cmd.id())
            .map(|knobs_data| knobs_data.passed_data.clone())
            .unwrap_or_default();
        YoleckKnobHandle {
            cmd,
            is_new,
            passed,
        }
    }
}
