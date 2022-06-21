use std::any::Any;
use std::hash::{Hash, BuildHasher, Hasher};

use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
use bevy::utils::HashMap;

#[derive(Default)]
pub struct KnobsCache {
    by_key_hash: HashMap<u64, Vec<CachedKnob>>,
}

struct CachedKnob {
    key: Box<dyn Any>,
    entity: Entity,
    keep_alive: bool,
}

impl KnobsCache {
    pub fn access<'w, 's, 'a, K: 'static + Hash + Eq>(&mut self, key: K, commands: &'a mut Commands<'w, 's>) -> KnobFromCache<'w, 's, 'a> {
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
                    }
                }
            }
        }
        let cmd = commands.spawn();
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

    pub fn drain(self) -> impl Iterator<Item = Entity> {
        self.by_key_hash.into_iter().flat_map(|(_, entries)| entries.into_iter().map(|entry| entry.entity))
    }
}

pub struct KnobFromCache<'w, 's, 'a> {
    pub cmd: EntityCommands<'w, 's, 'a>,
    pub is_new: bool,
}
