use crate::audio::{initialise_audio, Sounds};
use crate::components::editor::{
    CursorWasInThisEntity, EditorCursor, EditorFlag, EditorState, InsertionGameObject,
    RealCursorPosition,
};
use crate::components::game::{CameraTarget, Player, SerializedObject, SpriteRenderData};
use crate::components::game::{CollisionEvent, Health, Invincibility, SerializedObjectType};
use crate::components::graphics::SpriteSheetType;
use crate::components::graphics::{AnimationCounter, PulseAnimation, Scale};
use crate::components::physics::{
    Collidee, CollisionSideOfBlock, GravityDirection, Grounded, PlatformCollisionPoints,
    PlatformCuboid, Position, Sticky, Velocity,
};
use crate::events::Events;
use crate::level::Level;
use crate::states::pizzatopia;
use crate::states::pizzatopia::TILE_WIDTH;
use crate::states::pizzatopia::{get_camera_center, MyEvents, Pizzatopia};
use crate::systems;
use crate::systems::console::ConsoleInputSystem;
use crate::systems::editor::{
    CursorPositionSystem, CursorSizeSystem, EditorButtonEventSystem, EditorEventHandlingSystem,
    EditorEvents,
};
use crate::systems::graphics::{CursorSpriteUpdateSystem, PulseAnimationSystem};
use crate::systems::physics::CollisionDirection;
use crate::utils::{Vec2, Vec3};
use amethyst::core::math::Vector3;
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
        ArcThreadPool, EventReader, Hidden, SystemDesc, Time,
    },
    derive::EventReader,
    ecs::prelude::{
        Component, DenseVecStorage, Dispatcher, DispatcherBuilder, Entity, Join, WriteStorage,
    },
    input::{is_key_down, InputHandler, StringBindings, VirtualKeyCode},
    prelude::*,
    renderer::{
        debug_drawing::{DebugLines, DebugLinesComponent, DebugLinesParams},
        palette::{LinSrgba, Srgb, Srgba},
        rendy::{
            hal::image::{Filter, SamplerInfo, WrapMode},
            texture::image::{ImageTextureConfig, Repr, TextureKind},
        },
        resources::Tint,
        Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture, Transparent,
    },
    ui::{RenderUi, UiBundle, UiCreator, UiEvent, UiFinder, UiText},
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    winit::Event,
};
use log::{error, info, warn};
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::io;
use std::time::{Duration, Instant};

pub const EDITOR_GRID_SIZE: f32 = TILE_WIDTH / 2.0;

pub(crate) struct Editor<'a, 'b> {
    dispatcher: Option<Dispatcher<'a, 'b>>,
    time_start: Instant,
    prev_camera_target: CameraTarget,
}

impl Default for Editor<'_, '_> {
    fn default() -> Self {
        Editor {
            dispatcher: None,
            time_start: Instant::now(),
            prev_camera_target: CameraTarget::default(),
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for Editor<'_, '_> {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        data.world.insert(DebugLines::new());
        data.world.insert(DebugLinesParams { line_width: 2.0 });

        data.world
            .insert(InsertionGameObject(SerializedObject::default()));
        data.world.insert(EditorState::EditMode);

        // setup dispatcher
        let mut dispatcher = Editor::create_dispatcher(data.world);
        dispatcher.setup(data.world);
        self.dispatcher = Some(dispatcher);

        self.prev_camera_target = self.change_camera_target(data.world, CameraTarget::Cursor);

        // data.world.insert(EventChannel::<EditorEvents>::new());

        self.time_start = Instant::now();

        Self::initialize_cursor(data.world);

        Self::set_instance_entities_transparency(data.world, 0.5);
        Self::set_editor_entities_hidden_flag(data.world, false);
    }

    fn on_stop(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        Self::set_instance_entities_transparency(data.world, 1.0);
        Self::set_editor_entities_hidden_flag(data.world, true);

        self.change_camera_target(data.world, self.prev_camera_target);

        // Clean up cursor
        let mut to_remove = Vec::new();
        for (entity, _) in (
            &data.world.entities(),
            &data.world.read_storage::<EditorCursor>(),
        )
            .join()
        {
            to_remove.push(entity);
        }
        data.world
            .delete_entities(to_remove.as_slice())
            .expect("Failed to delete cursor entities.");
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'s, 's>>,
        event: MyEvents,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        if let MyEvents::Window(_) = &event {
            let input = data.world.read_resource::<InputHandler<StringBindings>>();
            if input.action_is_down("exit").unwrap_or(false) {
                return Trans::Quit;
            } else if input.action_is_down("editor").unwrap_or(false) {
                if self.time_start.elapsed().as_millis() > 250 {
                    return Trans::Pop;
                }
            }
        }

        if let MyEvents::App(event) = &event {
            match event {
                Events::AddGameObject => {
                    let mut serialized_object =
                        data.world.read_resource::<InsertionGameObject>().0.clone();
                    Level::initialize_serialized_object(data.world, &mut serialized_object, false);
                }
                Events::DeleteGameObject(id) => {
                    Level::delete_entity(data.world, *id);
                }
                Events::SaveLevel => {
                    Level::save_level(data.world);
                }
                Events::ChangeInsertionGameObject(id) => {
                    let mod_id = id % 2;
                    match mod_id {
                        0 => {
                            data.world
                                .insert(InsertionGameObject(SerializedObject::default()));
                        }
                        1 => {
                            let mut result: SerializedObject = SerializedObject::default();
                            result.object_type = SerializedObjectType::Player {
                                is_player: Player(true),
                            };
                            result.sprite = Some(SpriteRenderData::new(SpriteSheetType::Didi, 0));
                            data.world.insert(InsertionGameObject(result));
                        }
                        _ => {
                            error!("Can't change to this GameObject: {:?}", id);
                        }
                    }
                }
                Events::SetInsertionGameObject(serialized_object) => {
                    data.world
                        .insert(InsertionGameObject(serialized_object.clone()));
                }
                Events::EntityToInsertionGameObject(id) => {
                    let serialized_object = Level::entity_to_serialized_object(data.world, *id);
                    data.world.insert(InsertionGameObject(serialized_object));
                }
                Events::Warp(_) => {}
                Events::Reset => {}
            }
        }

        // Necessary to record changes made to entities by events
        data.world.maintain();

        // Escape isn't pressed, so we stay in this `State`.
        Trans::None
    }

    fn update(
        &mut self,
        mut data: StateData<'_, GameData<'s, 's>>,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        data.data.update(&mut data.world);
        if let Some(dispatcher) = self.dispatcher.as_mut() {
            dispatcher.dispatch(&data.world);
        }

        Trans::None
    }
}

impl<'a, 'b> Editor<'a, 'b> {
    fn create_dispatcher(world: &mut World) -> Dispatcher<'a, 'b> {
        // Main logic
        let mut dispatcher_builder = DispatcherBuilder::new();
        dispatcher_builder.add(
            CursorPositionSystem::new(world),
            "cursor_position_system",
            &[],
        );
        dispatcher_builder.add(
            CursorSizeSystem,
            "cursor_size_system",
            &["cursor_position_system"],
        );

        // The event handling is all done at the end since entities are created and deleted lazily
        dispatcher_builder.add(
            ConsoleInputSystem,
            "console_input_system",
            &["cursor_size_system"],
        );
        dispatcher_builder.add(
            EditorButtonEventSystem::new(world),
            "editor_button_event_system",
            &["cursor_size_system"],
        );
        dispatcher_builder.add(
            EditorEventHandlingSystem::new(world),
            "editor_event_handling_system",
            &["editor_button_event_system", "console_input_system"],
        );

        // Graphics
        dispatcher_builder.add(
            systems::graphics::ScaleDrawUpdateSystem,
            "scale_draw_update_system",
            &["editor_event_handling_system"],
        );
        dispatcher_builder.add(
            CursorSpriteUpdateSystem,
            "cursor_sprite_update_system",
            &["editor_event_handling_system"],
        );
        dispatcher_builder.add(
            systems::game::CameraTargetSystem,
            "camera_target_system",
            &["editor_event_handling_system"],
        );
        dispatcher_builder.add(
            PulseAnimationSystem,
            "pulse_animation_system",
            &["scale_draw_update_system"],
        );
        dispatcher_builder.add(
            systems::graphics::LerperSystem,
            "lerper_system",
            &["camera_target_system"],
        );
        dispatcher_builder.add(
            systems::graphics::PositionDrawUpdateSystem,
            "position_draw_update_system",
            &["lerper_system"],
        );

        dispatcher_builder
            .with_pool((*world.read_resource::<ArcThreadPool>()).clone())
            .build()
    }

    fn change_camera_target(
        &mut self,
        world: &mut World,
        camera_target: CameraTarget,
    ) -> CameraTarget {
        let mut prev_target = CameraTarget::default();

        for (_, target) in (
            &world.read_storage::<Camera>(),
            &mut world.write_storage::<CameraTarget>(),
        )
            .join()
        {
            prev_target = *target;
            *target = camera_target;
        }
        prev_target
    }

    fn set_instance_entities_transparency(world: &mut World, transparency: f32) {
        // make entities transparent
        for (entity, _, _) in (
            &world.entities(),
            &world.read_storage::<SpriteRender>(),
            !&world.read_storage::<EditorFlag>(),
        )
            .join()
        {
            let storage = &mut world.write_storage::<Tint>();
            storage
                .insert(entity, Tint(Srgba::new(1.0, 1.0, 1.0, transparency)))
                .expect("Error inserting Tint to entity in editor mode");
        }
    }

    fn set_editor_entities_hidden_flag(world: &mut World, hide: bool) {
        if hide {
            for (entity, _, _) in (
                &world.entities(),
                &world.read_storage::<SpriteRender>(),
                &world.read_storage::<EditorFlag>(),
            )
                .join()
            {
                let storage = &mut world.write_storage::<Hidden>();
                storage
                    .insert(entity, Hidden)
                    .expect("Error inserting Hidden to entity in editor mode");
            }
        } else {
            for (entity, _, _) in (
                &world.entities(),
                &world.read_storage::<SpriteRender>(),
                &world.read_storage::<EditorFlag>(),
            )
                .join()
            {
                let storage = &mut world.write_storage::<Hidden>();
                storage.remove(entity);
            }
        }
    }

    fn initialize_cursor(world: &mut World) {
        let mut transform = Transform::default();
        let scale = Vec3::new(0.5, 0.5, 1.0);
        transform.set_scale(Vector3::new(scale.x, scale.y, scale.z));

        // Correctly position the tile.
        let mut pos = get_camera_center(world).to_vec3();
        pos.z = pizzatopia::DEPTH_EDITOR;
        let pos = Position(pos);

        let sprite_sheet = world.read_resource::<BTreeMap<u8, Handle<SpriteSheet>>>()
            [&(SpriteSheetType::Tiles as u8)]
            .clone();
        // Assign the sprite
        let sprite_render = SpriteRender {
            sprite_sheet: sprite_sheet.clone(),
            sprite_number: 4,
        };

        // Create cursor
        world
            .create_entity()
            .with(EditorFlag)
            .with(EditorCursor)
            .with(RealCursorPosition(pos.0.to_vec2()))
            .with(PulseAnimation::default())
            .with(Scale(Vec2::new(scale.x, scale.y)))
            .with(CursorWasInThisEntity(None))
            .with(transform.clone())
            .with(sprite_render.clone())
            .with(pos.clone())
            .with(Transparent)
            .build();
    }
}
