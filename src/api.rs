use std::any::TypeId;
use std::marker::PhantomData;

use bevy::ecs::system::EntityCommands;
use bevy::prelude::AssetServer;
use bevy::utils::HashMap;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::{BoxedAny, YoleckTypeHandlerFor, YoleckTypeHandlerTrait};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum YoleckEditorState {
    EditorActive,
    GameActive,
}

#[derive(Clone, Copy)]
pub(crate) enum PopulateReason {
    EditorInit,
    EditorUpdate,
    RealGame,
}

pub struct YoleckPopulateContext<'a> {
    pub asset_server: &'a AssetServer,
    pub(crate) reason: PopulateReason,
    // I may add stuff that need 'a later, and I don't want to change the signature
    pub(crate) _phantom_data: PhantomData<&'a ()>,
}

impl<'a> YoleckPopulateContext<'a> {
    pub fn is_in_editor(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => true,
            PopulateReason::RealGame => false,
        }
    }

    pub fn is_first_time(&self) -> bool {
        match self.reason {
            PopulateReason::EditorInit => true,
            PopulateReason::EditorUpdate => false,
            PopulateReason::RealGame => true,
        }
    }
}

pub struct YoleckEditContext<'a> {
    pub(crate) passed: &'a HashMap<TypeId, &'a BoxedAny>,
}

impl YoleckEditContext<'_> {
    pub fn get_passed_data<T: 'static>(&self) -> Option<&T> {
        if let Some(dynamic) = self.passed.get(&TypeId::of::<T>()) {
            dynamic.downcast_ref()
        } else {
            None
        }
    }
}

pub trait YoleckSource: 'static + Send + Sync + Serialize + for<'a> Deserialize<'a> {
    const NAME: &'static str;

    fn populate(&self, ctx: &YoleckPopulateContext, cmd: &mut EntityCommands);
    fn edit(&mut self, ctx: &YoleckEditContext, ui: &mut egui::Ui);

    fn handler() -> Box<dyn YoleckTypeHandlerTrait> {
        Box::new(YoleckTypeHandlerFor::<Self> {
            _phantom_data: Default::default(),
        })
    }
}
