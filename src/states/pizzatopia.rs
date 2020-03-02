use crate::audio::{initialise_audio, Sounds};
use crate::bundles::{GameLogicBundle, GraphicsBundle};
use crate::components::game::{CollisionEvent, Health, Invincibility, Resettable};
use crate::components::graphics::AnimationCounter;
use crate::components::physics::{
    Collidee, CollisionSideOfBlock, GravityDirection, Grounded, PlatformCollisionPoints,
    PlatformCuboid, Position, Sticky, Velocity,
};
use crate::components::player::Player;
use crate::events::Events;
use crate::level::Level;
use crate::states::pizzatopia::SpriteSheetType::{Character, Tiles};
use crate::systems;
use crate::systems::console::ConsoleInputSystem;
use crate::systems::physics::CollisionDirection;
use crate::utils::Vec2;
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, ProgressCounter, RonFormat, Source,
    },
    core::{
        bundle::SystemBundle,
        ecs::{Read, SystemData, World},
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        transform::Transform,
        ArcThreadPool, EventReader, SystemDesc, Time,
    },
    derive::EventReader,
    ecs::prelude::{Component, DenseVecStorage, Dispatcher, DispatcherBuilder, Entity, Join},
    input::{is_key_down, InputHandler, StringBindings, VirtualKeyCode},
    prelude::*,
    renderer::{
        rendy::{
            hal::image::{Filter, SamplerInfo, WrapMode},
            texture::image::{ImageTextureConfig, Repr, TextureKind},
        },
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
    },
    ui::{RenderUi, UiBundle, UiCreator, UiEvent, UiFinder, UiText},
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    winit::Event,
};
use log::info;
use log::warn;
use std::borrow::Borrow;
use std::io;

pub const CAM_HEIGHT: f32 = TILE_HEIGHT * 12.0;
pub const CAM_WIDTH: f32 = TILE_WIDTH * 16.0;

pub const TILE_WIDTH: f32 = 128.0;
pub const TILE_HEIGHT: f32 = 128.0;

pub const MAX_FALL_SPEED: f32 = 20.0;
pub const MAX_RUN_SPEED: f32 = 20.0;

pub const FRICTION: f32 = 0.90;

#[repr(u8)]
#[derive(Clone)]
pub enum SpriteSheetType {
    Tiles = 0,
    Character,
}

#[derive(Debug, EventReader, Clone)]
#[reader(MyEventReader)]
pub enum MyEvents {
    Window(Event),
    Ui(UiEvent),
    App(Events),
}

pub(crate) struct Pizzatopia<'a, 'b> {
    fps_display: Option<Entity>,
    dispatcher: Option<Dispatcher<'a, 'b>>,
}

impl Default for Pizzatopia<'_, '_> {
    fn default() -> Self {
        Pizzatopia {
            fps_display: None,
            dispatcher: None,
        }
    }
}

impl Pizzatopia<'_, '_> {
    fn initialize_level(&mut self, world: &mut World, resetting: bool) {
        // remove entities to be reset
        let mut to_remove = Vec::new();
        for (entity, reset) in (&world.entities(), &world.read_storage::<Resettable>()).join() {
            to_remove.push(entity);
        }
        world
            .delete_entities(to_remove.as_slice())
            .expect("Failed to delete entities for reset.");

        // add the entities for the level
        initialise_actor(Vec2::new(CAM_WIDTH / 2.0, CAM_HEIGHT / 2.0), true, world);
        initialise_actor(
            Vec2::new(CAM_WIDTH / 2.0 - (TILE_HEIGHT * 2.0), CAM_HEIGHT / 2.0),
            false,
            world,
        );
        if !resetting {
            initialise_playground(world);
            initialise_camera(world);
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for Pizzatopia<'_, '_> {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        let mut world = data.world;
        world.register::<Resettable>();

        // setup dispatcher
        let mut dispatcher_builder = DispatcherBuilder::new();
        dispatcher_builder.add(ConsoleInputSystem, "console_input_system", &[]);
        dispatcher_builder.add(
            systems::PlayerInputSystem,
            "player_input_system",
            &["console_input_system"],
        );
        dispatcher_builder.add(
            systems::physics::ActorCollisionSystem,
            "actor_collision_system",
            &[],
        );
        dispatcher_builder.add(
            systems::physics::ApplyGravitySystem,
            "apply_gravity_system",
            &[],
        );
        dispatcher_builder.add(
            systems::physics::PlatformCollisionSystem,
            "platform_collision_system",
            &[
                "player_input_system",
                "apply_gravity_system",
                "actor_collision_system",
            ],
        );
        dispatcher_builder.add(
            systems::physics::ApplyCollisionSystem,
            "apply_collision_system",
            &["platform_collision_system"],
        );
        dispatcher_builder.add(
            systems::physics::ApplyVelocitySystem,
            "apply_velocity_system",
            &["apply_collision_system"],
        );
        dispatcher_builder.add(
            systems::physics::ApplyStickySystem,
            "apply_sticky_system",
            &["apply_velocity_system"],
        );
        // register a bundle to the builder
        GameLogicBundle::default()
            .build(&mut world, &mut dispatcher_builder)
            .expect("Failed to register GameLogic bundle.");
        GraphicsBundle::default()
            .build(&mut world, &mut dispatcher_builder)
            .expect("Failed to register Graphics bundle.");

        let mut dispatcher = dispatcher_builder
            .with_pool((*world.read_resource::<ArcThreadPool>()).clone())
            .build();
        dispatcher.setup(world);
        self.dispatcher = Some(dispatcher);

        self.initialize_level(world, false);

        world.exec(|mut creator: UiCreator<'_>| {
            let mut progress = ProgressCounter::new();
            creator.create("ui/fps.ron", &mut progress);
        });
    }

    fn handle_event(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
        event: MyEvents,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        let world = &mut data.world;
        if let MyEvents::Window(event) = &event {
            let input = world.read_resource::<InputHandler<StringBindings>>();
            if input.action_is_down("exit").unwrap_or(false) {
                return Trans::Quit;
            }
        }

        if let MyEvents::App(event) = &event {
            match event {
                Events::Reset => {
                    println!("Resetting map...");
                    self.initialize_level(world, true);
                }
                _ => {}
            }
        }

        if let MyEvents::Ui(event) = &event {
            println!("Ui event triggered!");
        }

        // Escape isn't pressed, so we stay in this `State`.
        Trans::None
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        if let Some(dispatcher) = self.dispatcher.as_mut() {
            dispatcher.dispatch(&mut data.world);
        }

        if self.fps_display.is_none() {
            data.world.exec(|finder: UiFinder<'_>| {
                if let Some(entity) = finder.find("fps_text") {
                    self.fps_display = Some(entity);
                }
            });
        }
        let mut ui_text = data.world.write_storage::<UiText>();
        {
            if let Some(fps_display) = self.fps_display.and_then(|entity| ui_text.get_mut(entity)) {
                if data.world.read_resource::<Time>().frame_number() % 20 == 0 {
                    let fps = data.world.read_resource::<FpsCounter>().sampled_fps();
                    fps_display.text = format!("FPS: {:.*}", 2, fps);
                }
            }
        }
        Trans::None
    }
}

/// Initialises the ground.
fn initialise_ground(world: &mut World, pos: Vec2) {
    let tile_size = (*world.read_resource::<Handle<Prefab<PlatformCuboid>>>()).clone();

    let transform = Transform::default();

    // Correctly position the tile.
    let pos = Position(pos);

    let sprite_sheet = world.read_resource::<Vec<Handle<SpriteSheet>>>()[Tiles as usize].clone();
    // Assign the sprite
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet.clone(),
        sprite_number: 0, // grass is the first sprite in the sprite_sheet
    };

    world
        .create_entity()
        .with(tile_size.clone())
        //.with(PlatformCuboid::new())
        .with(pos)
        .with(transform)
        .with(sprite_render.clone())
        .build();
}

fn initialise_playground(world: &mut World) {
    let tiles;
    {
        let asset = &world.read_resource::<AssetStorage<Level>>();
        let level = asset
            .get(&world.read_resource::<Handle<Level>>().clone())
            .expect("Expected level to be loaded.");
        tiles = level.tiles.clone();
    }

    for tile in tiles {
        initialise_ground(world, tile);
    }
}

/// Initialises one tile.
fn initialise_actor(pos: Vec2, player: bool, world: &mut World) {
    let mut transform = Transform::default();

    // Correctly position the paddles.
    transform.set_translation_xyz(pos.x, pos.y, 0.0);

    let sprite_sheet =
        world.read_resource::<Vec<Handle<SpriteSheet>>>()[Character as usize].clone();
    // Assign the sprite
    let sprite_render = SpriteRender {
        sprite_sheet: sprite_sheet.clone(),
        sprite_number: 1, // grass is the first sprite in the sprite_sheet
    };

    if player {
        world
            .create_entity()
            .with(Resettable)
            .with(transform)
            .with(sprite_render.clone())
            .with(AnimationCounter(0))
            .with(Player)
            .with(Grounded(false))
            .with(Position(Vec2::new(pos.x, pos.y)))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            //.with(PlatformCollisionPoints::vertical_line(TILE_HEIGHT / 2.0))
            .with(PlatformCollisionPoints::square(TILE_HEIGHT / 2.0))
            .with(Sticky(false))
            .with(GravityDirection(CollisionDirection::FromTop))
            .with(Collidee::new())
            .with(Health(5))
            .with(Invincibility(0))
            .build();
    } else {
        world
            .create_entity()
            .with(Resettable)
            .with(transform)
            .with(sprite_render.clone())
            .with(AnimationCounter(0))
            .with(Grounded(false))
            .with(Position(Vec2::new(pos.x, pos.y)))
            .with(Velocity(Vec2::new(0.0, 0.0)))
            //.with(PlatformCollisionPoints::vertical_line(TILE_HEIGHT / 2.0))
            .with(PlatformCollisionPoints::triangle(TILE_HEIGHT / 2.0))
            .with(Collidee::new())
            .build();
    }
}

fn initialise_camera(world: &mut World) {
    // Setup camera in a way that our screen covers whole arena and (0, 0) is in the bottom left.
    let mut transform = Transform::default();
    transform.set_translation_xyz(CAM_WIDTH * 0.5, CAM_HEIGHT * 0.5, 1.0);

    world
        .create_entity()
        .with(Camera::standard_2d(CAM_WIDTH, CAM_HEIGHT))
        .with(transform)
        .build();
}
