#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use amethyst::audio::AudioBundle;
use amethyst::input::{InputBundle, StringBindings};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, ProgressCounter, RonFormat, Source,
    },
    core::transform::TransformBundle,
    ecs::prelude::{Entity, ReadExpect, SystemData},
    prelude::*,
    ui::{RenderUi, UiBundle, UiCreator, UiEvent, UiFinder, UiText},
    renderer::{
        plugins::{RenderFlat2D, RenderToWindow},
        types::DefaultBackend,
        RenderingBundle,
    },
    utils::{application_root_dir, fps_counter::{FpsCounter, FpsCounterBundle},},
    Error, Logger,
};
use log::info;

mod audio;
mod components;
mod events;
mod level;
mod pizzatopia;
mod systems;
mod utils;
use crate::audio::initialise_audio;
use crate::components::physics::PlatformCuboid;
use crate::level::Level;
use crate::pizzatopia::MyEventReader;
use crate::pizzatopia::{MyEvents, Pizzatopia};
use crate::systems::console::ConsoleInputSystem;
use crate::systems::game::EnemyCollisionSystem;
use crate::systems::game::EnemyCollisionSystemDesc;
use crate::systems::game::PlayerEventsSystemDesc;
use amethyst::{
    core::{
        bundle::SystemBundle,
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        SystemDesc,
    },
    derive::SystemDesc,
    ecs::{DispatcherBuilder, Read, System, World, Write},
    prelude::*,
};

#[derive(Debug)]
struct MyBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for MyBundle {
    fn build(
        self,
        world: &mut World,
        builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        builder.add(
            systems::game::InvincibilitySystem,
            "invincibility_system",
            &["apply_velocity_system"],
        );
        builder.add(
            EnemyCollisionSystemDesc::default().build(world),
            "enemy_collision_system",
            &["invincibility_system"],
        );
        builder.add(
            PlayerEventsSystemDesc::default().build(world),
            "player_events_system",
            &["enemy_collision_system"],
        );
        Ok(())
    }
}

#[derive(Debug)]
struct GraphicsBundle;

impl<'a, 'b> SystemBundle<'a, 'b> for GraphicsBundle {
    fn build(
        self,
        world: &mut World,
        builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        builder.add(
            systems::graphics::SpriteUpdateSystem,
            "sprite_update_system",
            &["apply_velocity_system"],
        );
        builder.add(
            systems::graphics::PositionDrawUpdateSystem,
            "position_draw_update_system",
            &["sprite_update_system"],
        );
        builder.add(
            systems::graphics::DeadDrawUpdateSystem,
            "dead_draw_update_system",
            &["position_draw_update_system"],
        );
        Ok(())
    }
}

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
        .with_bundle(input_bundle)?
        .with_bundle(AudioBundle::default())?
        .with_bundle(TransformBundle::new())?
        .with_bundle(UiBundle::<StringBindings>::new())?
        .with_bundle(FpsCounterBundle::default())?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                // The RenderToWindow plugin provides all the scaffolding for opening a window and drawing on it
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderFlat2D::default())
                .with_plugin(RenderUi::default()),
        )?
        .with(Processor::<Level>::new(), "", &[])
        .with(
            ConsoleInputSystem,
            "console_input_system",
            &["input_system"],
        )
        .with(
            systems::PlayerInputSystem,
            "player_input_system",
            &["input_system", "console_input_system"],
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
            systems::physics::ApplyStickySystem,
            "apply_sticky_system",
            &["apply_velocity_system"],
        )
        .with_bundle(MyBundle)?
        .with_bundle(GraphicsBundle)?;

    let assets_dir = app_root.join("assets");

    let mut game = CoreApplication::<_, MyEvents, MyEventReader>::new(
        assets_dir,
        LoadingState {
            progress_counter: ProgressCounter::new(),
            level_handle: None,
            platform_size_prefab_handle: None,
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
    platform_size_prefab_handle: Option<Handle<Prefab<PlatformCuboid>>>,
}

impl<'s> State<GameData<'s, 's>, MyEvents> for LoadingState {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        let StateData { world, .. } = data;
        {
            initialise_audio(world);
        }
        let platform_size_prefab_handle =
            world.exec(|loader: PrefabLoader<'_, PlatformCuboid>| {
                loader.load("prefab/tile_size.ron", RonFormat, ())
            });

        let level_resource = &world.read_resource::<AssetStorage<Level>>();
        let level_handle = world.read_resource::<Loader>().load(
            "levels/level0.ron", // Here we load the associated ron file
            RonFormat,
            &mut self.progress_counter,
            &level_resource,
        );

        self.platform_size_prefab_handle = Some(platform_size_prefab_handle);
        self.level_handle = Some(level_handle);
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        if self.progress_counter.is_complete() {
            Trans::Switch(Box::new(Pizzatopia {
                level_handle: self.level_handle.clone().unwrap(),
                platform_size_prefab_handle: self.platform_size_prefab_handle.clone().unwrap(),
                spritesheets: Vec::new(),
                fps_display: None,
                load_ui: None,
            }))
        } else {
            Trans::None
        }
    }
}
