#![feature(clamp)]
#![allow(dead_code)]
#![allow(unused_imports)]
use amethyst::audio::AudioBundle;
use amethyst::input::{InputBundle, StringBindings};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, ProgressCounter, RonFormat, Source,
    },
    core::transform::TransformBundle,
    core::{
        bundle::SystemBundle,
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        SystemDesc,
    },
    derive::SystemDesc,
    ecs::prelude::{Entity, ReadExpect, SystemData},
    ecs::{DispatcherBuilder, Read, System, World, Write},
    prelude::*,
    renderer::{
        plugins::{RenderDebugLines, RenderFlat2D, RenderToWindow},
        types::DefaultBackend,
        ImageFormat, RenderingBundle, SpriteSheet, Texture,
    },
    ui::{RenderUi, UiBundle, UiCreator, UiEvent, UiFinder, UiText},
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    Error, Logger,
};
use log::info;

mod audio;
mod bundles;
mod components;
mod events;
mod level;
mod states;
mod systems;
mod ui;
mod utils;
use crate::audio::initialise_audio;
use crate::bundles::{GameLogicBundle, GraphicsBundle};
use crate::components::physics::PlatformCuboid;
use crate::level::Level;
use crate::states::loading::{AssetsDir, LoadingState};
use crate::states::pizzatopia::MyEventReader;
use crate::states::pizzatopia::{MyEvents, Pizzatopia};
use crate::systems::console::ConsoleInputSystem;
use crate::systems::game::EnemyCollisionSystem;

fn main() -> amethyst::Result<()> {
    // Logging for GL stuff
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir()?;

    let config_dir = app_root.join("config");
    let display_config_path = config_dir.join("display.ron");
    let binding_path = app_root.join("config").join("bindings.ron");
    let input_bundle =
        InputBundle::<StringBindings>::new().with_bindings_from_file(binding_path)?;

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
                .with_plugin(RenderUi::default())
                .with_plugin(RenderDebugLines::default()),
        )?
        .with(Processor::<Level>::new(), "", &[]);
    let assets_dir = app_root.join("assets");

    let mut game = CoreApplication::<_, MyEvents, MyEventReader>::new(
        assets_dir,
        LoadingState::default(),
        game_data,
    )?;
    game.run();

    Ok(())
}
