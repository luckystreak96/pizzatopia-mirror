#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use amethyst::input::{InputBundle, StringBindings};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, PrefabLoaderSystemDesc, ProcessingState,
        Processor, ProgressCounter, RonFormat, Source,
    },
    core::transform::TransformBundle,
    ecs::prelude::{ReadExpect, SystemData},
    prelude::*,
    renderer::{
        plugins::{RenderFlat2D, RenderToWindow},
        types::DefaultBackend,
        RenderingBundle,
    },
    utils::application_root_dir,
    Logger,
};
use log::info;

mod components;
mod level;
mod pizzatopia;
mod systems;
mod utils;
use crate::components::physics::PlatformCuboid;
use crate::level::Level;
use crate::pizzatopia::Pizzatopia;

fn main() -> amethyst::Result<()> {
    // Logging for GL stuff
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let config_dir = app_root.join("config");
    let display_config_path = config_dir.join("display.ron");
    let binding_path = app_root.join("config").join("bindings.ron");
    let input_bundle =
        InputBundle::<StringBindings>::new().with_bindings_from_file(binding_path)?;

    let world = World::new();

    let game_data = GameDataBuilder::default()
        .with_system_desc(PrefabLoaderSystemDesc::<PlatformCuboid>::default(), "", &[])
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                // The RenderToWindow plugin provides all the scaffolding for opening a window and drawing on it
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderFlat2D::default()),
        )?
        .with_bundle(input_bundle)?
        .with_bundle(TransformBundle::new())?
        .with(Processor::<Level>::new(), "", &[])
        .with(
            systems::PlayerInputSystem,
            "player_input_system",
            &["input_system"],
        )
        .with(
            systems::physics::ActorCollisionSystem,
            "actor_collision_system",
            &[],
        )
        .with(
            systems::physics::ApplyGravitySystem,
            "apply_gravity_system",
            &[],
        )
        .with(
            systems::physics::PlatformCollisionSystem,
            "platform_collision_system",
            &[
                "player_input_system",
                "apply_gravity_system",
                "actor_collision_system",
            ],
        )
        .with(
            systems::physics::ApplyCollisionSystem,
            "apply_collision_system",
            &["platform_collision_system"],
        )
        .with(
            systems::physics::ApplyVelocitySystem,
            "apply_velocity_system",
            &["apply_collision_system"],
        )
        .with(
            systems::graphics::SpriteUpdateSystem,
            "sprite_update_system",
            &["apply_velocity_system"],
        )
        .with(
            systems::graphics::PositionDrawUpdateSystem,
            "position_draw_update_system",
            &["apply_velocity_system"],
        );

    let assets_dir = app_root.join("assets");

    let mut game = Application::new(
        assets_dir,
        LoadingState {
            progress_counter: ProgressCounter::new(),
            level_handle: None,
        },
        game_data,
    )?;
    game.run();

    Ok(())
}

pub struct LoadingState {
    /// Tracks loaded assets.
    progress_counter: ProgressCounter,
    /// Handle to the energy blast.
    level_handle: Option<Handle<Level>>,
}

impl SimpleState for LoadingState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let level_resource = &data.world.read_resource::<AssetStorage<Level>>();
        let level_handle = data.world.read_resource::<Loader>().load(
            "levels/level0.ron", // Here we load the associated ron file
            RonFormat,
            &mut self.progress_counter,
            &level_resource,
        );

        self.level_handle = Some(level_handle);
    }

    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        if self.progress_counter.is_complete() {
            Trans::Switch(Box::new(Pizzatopia {
                level_handle: self.level_handle.clone().unwrap(),
                spritesheets: Vec::new(),
            }))
        } else {
            Trans::None
        }
    }
}
