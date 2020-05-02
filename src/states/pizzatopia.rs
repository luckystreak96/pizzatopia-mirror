use crate::audio::{initialise_audio, Sounds};
use crate::bundles::{GameLogicBundle, GraphicsBundle};
use crate::components::editor::{EditorFlag, InstanceEntityId, SizeForEditorGrid};
use crate::components::game::{CameraTarget, CollisionEvent, GameObject, Health, Invincibility};
use crate::components::game::{Player, Resettable};
use crate::components::graphics::{AnimationCounter, CameraLimit, Lerper};
use crate::components::physics::{
    Collidee, CollisionSideOfBlock, GravityDirection, Grounded, PlatformCollisionPoints,
    PlatformCuboid, Position, Sticky, Velocity,
};
use crate::events::Events;
use crate::level::Level;
use crate::level::Tile;
use crate::states::editor::Editor;
use crate::states::pizzatopia::SpriteSheetType::{Character, Tiles};
use crate::systems;
use crate::systems::console::ConsoleInputSystem;
use crate::systems::physics::CollisionDirection;
use crate::utils::{Vec2, Vec3};
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
use std::thread::park_timeout;
use std::time::Instant;

pub const CAM_HEIGHT: f32 = TILE_HEIGHT * 12.0;
pub const CAM_WIDTH: f32 = TILE_WIDTH * 16.0;

pub const DEPTH_TILES: f32 = 1.0;
pub const DEPTH_ACTORS: f32 = DEPTH_TILES + 1.0;
pub const DEPTH_EDITOR: f32 = DEPTH_ACTORS + 1.0;
pub const DEPTH_UI: f32 = DEPTH_EDITOR + 1.0;

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
    Snap,
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
    time_start: Instant,
}

impl Default for Pizzatopia<'_, '_> {
    fn default() -> Self {
        Pizzatopia {
            fps_display: None,
            dispatcher: None,
            time_start: Instant::now(),
        }
    }
}

impl Pizzatopia<'_, '_> {
    fn initialize_level(&mut self, world: &mut World) {
        Level::load_level(world);
        initialise_camera(world);
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for Pizzatopia<'_, '_> {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        data.world.register::<GameObject>();
        data.world.register::<GameObject>();
        data.world.register::<Resettable>();
        data.world.register::<EditorFlag>();
        data.world.register::<CameraTarget>();
        data.world.register::<Tile>();
        // Created in Pizzatopia and system in Editor
        data.world.register::<SizeForEditorGrid>();
        // Created in Pizzatopia and system in Editor
        data.world.register::<InstanceEntityId>();

        // setup dispatcher
        let mut dispatcher = Pizzatopia::create_pizzatopia_dispatcher(data.world);
        dispatcher.setup(data.world);
        self.dispatcher = Some(dispatcher);

        initialise_camera(data.world);
        Level::load_level(data.world);

        data.world.exec(|mut creator: UiCreator<'_>| {
            let mut progress = ProgressCounter::new();
            creator.create("ui/fps.ron", &mut progress);
        });
    }

    fn on_resume(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        self.time_start = Instant::now();
        Level::calculate_camera_limits(data.world);
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
            } else if input.action_is_down("editor").unwrap_or(false) {
                if self.time_start.elapsed().as_millis() > 250 {
                    return Trans::Push(Box::new(Editor::default()));
                }
            }
        }

        if let MyEvents::App(event) = &event {
            match event {
                Events::Reset => {
                    println!("Resetting map...");
                    Level::reinitialize_level(world);
                }
                Events::ChangeInsertionGameObject(_) => {}
                Events::SetInsertionGameObject(_) => {}
                Events::SaveLevel => {}
                Events::Warp(_) => {}
                Events::AddGameObject(_) => {}
                Events::DeleteGameObject(_) => {}
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

fn initialise_camera(world: &mut World) {
    // Setup camera in a way that our screen covers whole arena and (0, 0) is in the bottom left.
    let mut transform = Transform::default();
    let pos = Position(Vec3::new(CAM_WIDTH * 0.5, CAM_HEIGHT * 0.5, 2000.0));
    transform.set_translation_xyz(pos.0.x, pos.0.y, pos.0.z);

    world
        .create_entity()
        .with(Camera::standard_2d(CAM_WIDTH, CAM_HEIGHT))
        .with(transform)
        .with(pos)
        .with(CameraTarget::Player)
        .with(CameraLimit::default())
        .with(Lerper::default())
        .build();
}

pub fn get_camera_center(world: &mut World) -> Vec2 {
    for (camera, transform) in (
        &world.read_storage::<Camera>(),
        &world.read_storage::<Transform>(),
    )
        .join()
    {
        return Vec2::new(transform.translation().x, transform.translation().y);
    }
    return Vec2::new(0.0, 0.0);
}

impl<'a, 'b> Pizzatopia<'a, 'b> {
    fn create_pizzatopia_dispatcher(world: &mut World) -> Dispatcher<'a, 'b> {
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
        dispatcher_builder.add(
            systems::game::CameraTargetSystem,
            "camera_target_system",
            &["apply_velocity_system"],
        );
        dispatcher_builder.add(
            systems::graphics::LerperSystem,
            "lerper_system",
            &["camera_target_system"],
        );
        dispatcher_builder.add(
            systems::graphics::CameraEdgeClampSystem,
            "camera_edge_clamp_system",
            &["lerper_system"],
        );
        // register a bundle to the builder
        GameLogicBundle::default()
            .build(world, &mut dispatcher_builder)
            .expect("Failed to register GameLogic bundle.");
        GraphicsBundle::default()
            .build(world, &mut dispatcher_builder)
            .expect("Failed to register Graphics bundle.");

        dispatcher_builder
            .with_pool((*world.read_resource::<ArcThreadPool>()).clone())
            .build()
    }
}
