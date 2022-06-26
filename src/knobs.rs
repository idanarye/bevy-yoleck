use std::any::Any;
use std::hash::{BuildHasher, Hash, Hasher};

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Component)]
pub struct YoleckKnob;

#[derive(Default)]
pub struct YoleckKnobsCache {
    by_key_hash: HashMap<u64, Vec<CachedKnob>>,
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
        let mut cmd = commands.spawn();
        cmd.insert(YoleckKnob);
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
