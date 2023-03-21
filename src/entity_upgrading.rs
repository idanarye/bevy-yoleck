use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::YoleckRawLevel;

pub struct YoleckEntityUpgradingPlugin {
    pub app_format_version: usize,
}

impl Plugin for YoleckEntityUpgradingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(YoleckEntityUpgrading {
            app_format_version: self.app_format_version,
            upgrade_functions: Default::default(),
        });
    }
}

#[derive(Resource)]
pub(crate) struct YoleckEntityUpgrading {
    pub app_format_version: usize,
    #[allow(clippy::type_complexity)]
    pub upgrade_functions:
        BTreeMap<usize, Vec<Box<dyn 'static + Send + Sync + Fn(&str, &mut serde_json::Value)>>>,
}

impl YoleckEntityUpgrading {
    pub fn upgrade_raw_level_file(&self, levels_file: &mut YoleckRawLevel) {
        let first_target_version = levels_file.0.app_format_version + 1;
        for (target_version, upgrade_functions) in
            self.upgrade_functions.range(first_target_version..)
        {
            for entity in levels_file.2.iter_mut() {
                for function in upgrade_functions.iter() {
                    function(&entity.header.type_name, &mut entity.data);
                }
            }
            levels_file.0.app_format_version = *target_version;
        }
    }
}
