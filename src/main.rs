#![allow(dead_code)]
use amethyst::input::{InputBundle, StringBindings};
use amethyst::{
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

#[macro_use]
extern crate smart_default;

mod components;
mod pizzatopia;
mod systems;
mod utils;
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

    let mut world = World::new();

    let game_data = GameDataBuilder::default()
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                // The RenderToWindow plugin provides all the scaffolding for opening a window and drawing on it
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderFlat2D::default()),
        )?
        .with_bundle(input_bundle)?
        .with_bundle(TransformBundle::new())?
        .with(systems::PlayerSystem, "player_system", &["input_system"]);

    let assets_dir = app_root.join("assets");

    let mut game = Application::new(assets_dir, Pizzatopia, game_data)?;
    game.run();

    Ok(())
}
