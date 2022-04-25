use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiContext, egui};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum AppMode {
    Editor,
}

fn main() {
    let mut app = App::new();
    app.add_state(AppMode::Editor);
    app.add_plugins(DefaultPlugins);
    app.add_plugin(EguiPlugin);
    app.add_startup_system(setup_camera);
    app.add_startup_system(setup_entities); // TODO: replace with entity setup from data;
    app.add_startup_system(setup_yoleck_editing);
    app.add_system_set(SystemSet::on_update(AppMode::Editor).with_system(yoleck_editor.exclusive_system()));
    app.insert_resource(YoleckState {
        entity_being_edited: None,
        edit_functions: Default::default(),
    });
    app.run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn setup_entities(mut commands: Commands) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::RED,
                custom_size: Some(Vec2::new(20.0, 20.0)),
                ..Default::default()
            },
            transform: Transform::from_xyz(-50.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(YoleckOwned);

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::BLUE,
                custom_size: Some(Vec2::new(20.0, 20.0)),
                ..Default::default()
            },
            transform: Transform::from_xyz(50.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(YoleckOwned);
}

#[derive(Component)]
pub struct YoleckOwned;

fn yoleck_editor(world: &mut World) {
    let mut egui_context = world.resource_mut::<EguiContext>().clone();
    let egui_ctx = egui_context.ctx_mut();
    egui::Window::new("Level Editor").show(egui_ctx, |ui| {
        // let query_state = world.query::<EditBox>();
        world.resource_scope(|world, mut yoleck: Mut<YoleckState>| {
            for (entity, _) in world.query::<(Entity, &YoleckOwned)>().iter(world) {
                ui.selectable_value(&mut yoleck.entity_being_edited, Some(entity), format!("{:?}", entity));
            }
            if let Some(entity) = yoleck.entity_being_edited {
                for edit_function in yoleck.edit_functions.iter_mut() {
                    edit_function(ui, world, entity);
                }
            }
        });
    });
}

fn edit_box() -> impl Send + Sync + FnMut(&mut egui::Ui, &mut World, Entity) {
    let mut query_state = None;
    move |ui, world, entity| {
        let query_state = query_state.get_or_insert_with(|| world.query::<(&mut Transform,)>());
        if let Ok((transform,)) = query_state.get_mut(world, entity) {
            let transform = transform.into_inner();
            ui.add(egui::Slider::new(&mut transform.translation.y, -100.0..=100.0));
        }
    }
}

type YoleckEditFunction = Box<dyn Send + Sync + FnMut(&mut egui::Ui, &mut World, Entity)>;

struct YoleckState {
    entity_being_edited: Option<Entity>,
    edit_functions: Vec<YoleckEditFunction>,
}

fn setup_yoleck_editing(mut yoleck: ResMut<YoleckState>) {
    yoleck.edit_functions.push(Box::new(edit_box()));
}
