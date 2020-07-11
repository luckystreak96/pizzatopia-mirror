use crate::{
    audio::{initialise_audio, Sounds},
    components::{
        editor::{
            CursorWasInThisEntity, EditorCursor, EditorFlag, EditorState, InsertionGameObject,
            InstanceEntityId, RealCursorPosition, SizeForEditorGrid,
        },
        entity_builder::entity_builder,
        game::{
            CameraTarget, CollisionEvent, Health, Invincibility, Player, SerializedObject,
            SerializedObjectType, SpriteRenderData,
        },
        graphics::{AbsolutePositioning, AnimationCounter, PulseAnimation, Scale, SpriteSheetType},
        physics::{
            Collidee, CollisionSideOfBlock, GravityDirection, Grounded, PlatformCollisionPoints,
            PlatformCuboid, Position, Sticky, Velocity,
        },
    },
    events::Events,
    level::Level,
    states::{
        load_level::LoadLevelState,
        pizzatopia,
        pizzatopia::{get_camera_center, MyEvents, Pizzatopia, TILE_HEIGHT, TILE_WIDTH},
    },
    systems,
    systems::{
        console::ConsoleInputSystem,
        editor::{
            CursorPositionSystem, CursorSizeSystem, CursorStateSystem, EditorButtonEventSystem,
            EditorEventHandlingSystem, EditorEvents,
        },
        graphics::{CursorColorUpdateSystem, CursorSpriteUpdateSystem, PulseAnimationSystem},
        input::{InputManagementSystem, InputManager},
        physics::CollisionDirection,
    },
    ui::{
        current_actions::CurrentActionsUi,
        file_picker::FilePickerUi,
        tile_characteristics::{EditorFieldUiComponents, UiIndex},
        UiStack,
    },
    utils::{Vec2, Vec3},
};
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, Prefab, PrefabData, PrefabLoader,
        PrefabLoaderSystemDesc, ProcessingState, Processor, ProgressCounter, RonFormat, Source,
    },
    core::{
        ecs::{Read, World},
        frame_limiter::FrameRateLimitStrategy,
        math::Vector3,
        shrev::{EventChannel, ReaderId},
        transform::Transform,
        ArcThreadPool, Hidden, HiddenPropagate, Time,
    },
    derive::EventReader,
    ecs::prelude::{
        Component, DenseVecStorage, Dispatcher, DispatcherBuilder, Entity, Join, Storage,
        WriteStorage,
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
    shred::{Fetch, FetchMut},
    ui::{
        Anchor, FontAsset, RenderUi, TextEditing, TtfFormat, UiBundle, UiCreator, UiEvent,
        UiEventType, UiFinder, UiText, UiTransform,
    },
    utils::{
        application_root_dir,
        fps_counter::{FpsCounter, FpsCounterBundle},
    },
    winit::Event,
};
use log::{error, warn};

pub const EDITOR_GRID_SIZE: f32 = TILE_WIDTH / 2.0;

pub(crate) struct Editor<'a, 'b> {
    test_text: Option<Entity>,
    dispatcher: Option<Dispatcher<'a, 'b>>,
    prev_camera_target: CameraTarget,
}

impl Default for Editor<'_, '_> {
    fn default() -> Self {
        Editor {
            test_text: None,
            dispatcher: None,
            prev_camera_target: CameraTarget::default(),
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for Editor<'_, '_> {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        data.world.register::<HiddenPropagate>();

        if data.world.try_fetch::<InsertionGameObject>().is_none() {
            data.world
                .insert(InsertionGameObject(SerializedObject::default()));
        }
        data.world.insert(EditorState::EditMode);
        data.world.insert(UiIndex::default());
        let mut ui_stack = UiStack::default();
        ui_stack
            .stack
            .push(Box::new(CurrentActionsUi::new(data.world)));
        ui_stack
            .stack
            .push(Box::new(EditorFieldUiComponents::new(data.world)));
        data.world.insert(ui_stack);

        // setup dispatcher
        let mut dispatcher = Editor::create_dispatcher(data.world);
        dispatcher.setup(data.world);
        self.dispatcher = Some(dispatcher);

        self.prev_camera_target = self.change_camera_target(data.world, CameraTarget::Cursor);

        // data.world.insert(EventChannel::<EditorEvents>::new());

        entity_builder::initialize_cursor(data.world);

        Self::set_instance_entities_transparency(data.world, 0.5);
        Self::set_editor_entities_hidden_flag(data.world, false);
    }

    fn on_stop(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        Self::set_instance_entities_transparency(data.world, 1.0);
        Self::set_editor_entities_hidden_flag(data.world, true);

        self.change_camera_target(data.world, self.prev_camera_target);

        // Clean up cursor
        let mut to_remove: Vec<Entity> = Vec::new();
        for (entity, _) in (
            &data.world.entities(),
            &data.world.read_storage::<EditorCursor>(),
        )
            .join()
        {
            to_remove.push(entity);
        }
        for ui_component in data.world.read_resource::<UiStack>().stack.iter() {
            to_remove = ui_component.entities_to_remove(to_remove);
        }
        Self::remove_entities(to_remove, data.world);
    }

    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'s, 's>>,
        event: MyEvents,
    ) -> Trans<GameData<'s, 's>, MyEvents> {
        if let MyEvents::Window(_) = &event {
            let input = data.world.read_resource::<InputManager>();
            if input.action_status("exit").is_down {
                return Trans::Quit;
            } else if input.action_single_press("editor").is_down {
                return Trans::Pop;
            }
        }

        if let MyEvents::Ui(event) = &event {
            data.world.write_resource::<UiStack>().handle_ui_events(
                data.world,
                Some(event.clone()),
                None,
            );
        }

        if let MyEvents::App(event) = &event {
            data.world.write_resource::<UiStack>().handle_ui_events(
                data.world,
                None,
                Some(event.clone()),
            );
            match event {
                Events::AddGameObject => {
                    let mut serialized_object =
                        data.world.read_resource::<InsertionGameObject>().0.clone();
                    entity_builder::initialize_serialized_object(
                        data.world,
                        &mut serialized_object,
                        false,
                    );
                }
                Events::DeleteGameObject(id) => {
                    Self::delete_entity(data.world, *id);
                }
                Events::SaveLevel => {
                    Level::save_level(data.world);
                }
                Events::LoadLevel => {
                    return Trans::Sequence(vec![
                        Trans::Pop,
                        Trans::Switch(Box::new(LoadLevelState::default())),
                    ]);
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
                                is_player: Player(false),
                            };
                            result.sprite = Some(SpriteRenderData::new(SpriteSheetType::Snap, 0));
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
                    let serialized_object =
                        entity_builder::entity_to_serialized_object(data.world, *id);
                    data.world.insert(InsertionGameObject(serialized_object));
                }
                Events::OpenFilePickerUi => {
                    warn!("Opening file picker!");
                    let file_picker_ui = Box::new(FilePickerUi::new(data.world));
                    data.world
                        .write_resource::<UiStack>()
                        .stack
                        .push(file_picker_ui);
                }
                _ => {}
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

        let mut index = 0;
        let mut remove_indices = Vec::new();
        for ui in &mut data.world.write_resource::<UiStack>().stack {
            ui.update(data.world);
            if ui.should_destroy() {
                remove_indices.insert(0, index);
            }
            index += 1;
        }
        let mut entities = Vec::new();
        for index in remove_indices {
            entities =
                data.world.read_resource::<UiStack>().stack[index].entities_to_remove(entities);
            data.world.write_resource::<UiStack>().stack.remove(index);
        }
        Self::remove_entities(entities, data.world);

        Trans::None
    }
}

impl<'a, 'b> Editor<'a, 'b> {
    fn remove_entities(entities: Vec<Entity>, world: &mut World) {
        world
            .delete_entities(entities.as_slice())
            .expect("Failed to delete entities.");
    }

    fn create_dispatcher(world: &mut World) -> Dispatcher<'a, 'b> {
        // Main logic
        let mut dispatcher_builder = DispatcherBuilder::new();
        dispatcher_builder.add(InputManagementSystem, "input_management_system", &[]);
        dispatcher_builder.add(
            CursorPositionSystem,
            "cursor_position_system",
            &["input_management_system"],
        );
        dispatcher_builder.add(
            CursorSizeSystem,
            "cursor_size_system",
            &["cursor_position_system"],
        );
        dispatcher_builder.add(
            CursorStateSystem,
            "cursor_state_system",
            &["cursor_size_system"],
        );

        // The event handling is all done at the end since entities are created and deleted lazily
        dispatcher_builder.add(
            ConsoleInputSystem,
            "console_input_system",
            &["cursor_state_system"],
        );
        dispatcher_builder.add(
            EditorButtonEventSystem,
            "editor_button_event_system",
            &["cursor_state_system"],
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
            CursorColorUpdateSystem,
            "cursor_color_update_system",
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
            systems::graphics::AbsolutePositionUpdateSystem,
            "absolute_position_update_system",
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

    // Remove an entity and its instance entity if it is an editor entity
    fn delete_entity(world: &mut World, id: u32) {
        warn!("Deleting tile {:?}!", id);
        // Get the editor entity
        let editor_entity = world.entities().entity(id);

        // Delete the instance entity using editor entity
        if let Some(instance_id) = world.read_storage::<InstanceEntityId>().get(editor_entity) {
            if let Some(instance_id) = instance_id.0 {
                let instance_entity = world.entities().entity(instance_id);
                match world.entities().delete(instance_entity) {
                    Ok(_) => {}
                    Err(_) => error!("Error deleting instance entity."),
                }
            }
        }
        match world.entities().delete(editor_entity) {
            Ok(_) => {}
            Err(_) => error!("Error deleting editor entity."),
        }
    }
}
