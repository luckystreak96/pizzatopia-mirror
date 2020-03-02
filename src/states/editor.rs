use crate::components::physics::PlatformCuboid;
use crate::level::Level;
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
        EventReader, SystemDesc, Time,
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

// pub(crate) struct Editor<'a, 'b> {
//     level_handle: Handle<Level>,
//     platform_size_prefab_handle: Handle<Prefab<PlatformCuboid>>,
//     spritesheets: Vec<Handle<SpriteSheet>>,
//     fps_display: Option<Entity>,
//     dispatcher: Option<Dispatcher<'a, 'b>>,
// }
//
// impl Editor<'_, '_> {
//     pub(crate) fn new(
//         level: Handle<Level>,
//         platform_cuboid_handle: Handle<Prefab<PlatformCuboid>>,
//     ) -> Self {
//         Editor {
//             level_handle: level,
//             platform_size_prefab_handle: platform_cuboid_handle,
//             spritesheets: Vec::new(),
//             fps_display: None,
//             dispatcher: None,
//         }
//     }
//
//     fn load_sprite_sheets(&mut self, world: &mut World) {
//         self.spritesheets
//             .push(load_spritesheet(String::from("texture/tiles"), world));
//         self.spritesheets
//             .push(load_spritesheet(String::from("texture/spritesheet"), world));
//     }
//
//     fn initialize_level(&mut self, world: &mut World, resetting: bool) {
//         let tiles_sprite_sheet_handle = self.spritesheets[Tiles as usize].clone();
//         let actor_sprite_sheet_handle = self.spritesheets[Character as usize].clone();
//         let prefab_handle = self.platform_size_prefab_handle.clone();
//
//         // remove entities to be reset
//         let mut entities = Vec::new();
//         for (entity, reset) in (&world.entities(), &world.read_storage::<Resettable>()).join() {
//             entities.push(entity);
//         }
//         world
//             .delete_entities(entities.as_slice())
//             .expect("Failed to delete entities for reset.");
//
//         initialise_actor(
//             Vec2::new(CAM_WIDTH / 2.0, CAM_HEIGHT / 2.0),
//             true,
//             world,
//             actor_sprite_sheet_handle.clone(),
//         );
//         initialise_actor(
//             Vec2::new(CAM_WIDTH / 2.0 - (TILE_HEIGHT * 2.0), CAM_HEIGHT / 2.0),
//             false,
//             world,
//             actor_sprite_sheet_handle.clone(),
//         );
//         if !resetting {
//             initialise_playground(
//                 world,
//                 tiles_sprite_sheet_handle.clone(),
//                 prefab_handle,
//                 self.level_handle.clone(),
//             );
//             initialise_camera(world);
//         }
//     }
// }
//
// impl<'s> State<GameData<'s, 's>, MyEvents> for Pizzatopia<'_, '_> {
//     fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
//         let world = data.world;
//
//         world.register::<PlatformCuboid>();
//         world.register::<PlatformCollisionPoints>();
//         world.register::<Health>();
//         world.register::<Invincibility>();
//         world.register::<Resettable>();
//
//         self.load_sprite_sheets(world);
//         self.initialize_level(world, false);
//
//         world.exec(|mut creator: UiCreator<'_>| {
//             let mut progress = ProgressCounter::new();
//             creator.create("ui/fps.ron", &mut progress);
//         });
//     }
//
//     fn handle_event(
//         &mut self,
//         mut data: StateData<'_, GameData<'s, 's>>,
//         event: MyEvents,
//     ) -> Trans<GameData<'s, 's>, MyEvents> {
//         let world = &mut data.world;
//         if let MyEvents::Window(event) = &event {
//             let input = world.read_resource::<InputHandler<StringBindings>>();
//             if input.action_is_down("exit").unwrap_or(false) {
//                 return Trans::Quit;
//             }
//         }
//
//         if let MyEvents::App(event) = &event {
//             match event {
//                 Events::Reset => {
//                     println!("Resetting map...");
//                     self.initialize_level(world, true);
//                 }
//                 _ => {}
//             }
//         }
//
//         if let MyEvents::Ui(event) = &event {
//             println!("Ui event triggered!");
//         }
//
//         // Escape isn't pressed, so we stay in this `State`.
//         Trans::None
//     }
//
//     fn update(
//         &mut self,
//         mut data: StateData<'_, GameData<'s, 's>>,
//     ) -> Trans<GameData<'s, 's>, MyEvents> {
//         data.data.update(&mut data.world);
//         if self.fps_display.is_none() {
//             data.world.exec(|finder: UiFinder<'_>| {
//                 if let Some(entity) = finder.find("fps_text") {
//                     self.fps_display = Some(entity);
//                 }
//             });
//         }
//         let mut ui_text = data.world.write_storage::<UiText>();
//         {
//             if let Some(fps_display) = self.fps_display.and_then(|entity| ui_text.get_mut(entity)) {
//                 if data.world.read_resource::<Time>().frame_number() % 20 == 0 {
//                     let fps = data.world.read_resource::<FpsCounter>().sampled_fps();
//                     fps_display.text = format!("FPS: {:.*}", 2, fps);
//                 }
//             }
//         }
//         Trans::None
//     }
// }
