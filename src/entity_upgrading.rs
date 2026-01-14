use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::YoleckRawLevel;

/// Support upgrading of entities when the layout of the Yoleck entities and components change.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_yoleck::prelude::*;
/// # let mut app = App::new();
/// app.add_plugins(YoleckEntityUpgradingPlugin {
///     app_format_version: 5,
/// });
///
/// // The newest upgrade, from 4 to 5
/// app.add_yoleck_entity_upgrade_for(5, "Foo", |data| {
///     let mut old_data = data.remove("OldFooComponent").unwrap();
///     data["NewFooComponent"] = old_data;
/// });
///
/// // Some older upgrade, from 2 to 3
/// app.add_yoleck_entity_upgrade(3, |_type_name, data| {
///     if let Some(component_data) = data.get_mut("Bar") {
///         component_data["some_new_field"] = 42.into();
///     }
/// });
/// ```
pub struct YoleckEntityUpgradingPlugin {
    /// The current version of the app data.
    ///
    /// If `YoleckEntityUpgradingPlugin` is not added, the current version is considered 0
    /// ("unversioned")
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
    pub upgrade_functions: BTreeMap<
        usize,
        Vec<
            Box<
                dyn 'static
                    + Send
                    + Sync
                    + Fn(&str, &mut serde_json::Map<String, serde_json::Value>),
            >,
        >,
    >,
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
