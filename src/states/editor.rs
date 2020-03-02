use crate::audio::{initialise_audio, Sounds};
use crate::components::game::{CollisionEvent, Health, Invincibility, Resettable};
use crate::components::graphics::AnimationCounter;
use crate::components::physics::{
    Collidee, CollisionSideOfBlock, GravityDirection, Grounded, PlatformCollisionPoints,
    PlatformCuboid, Position, Sticky, Velocity,
};
use crate::components::player::Player;
use crate::events::Events;
use crate::level::Level;
use crate::states::pizzatopia::MyEvents;
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
use std::time::{Duration, Instant};

pub(crate) struct Editor<'a, 'b> {
    dispatcher: Option<Dispatcher<'a, 'b>>,
    time_start: Instant,
}

impl Default for Editor<'_, '_> {
    fn default() -> Self {
        Editor {
            dispatcher: None,
            time_start: Instant::now(),
        }
    }
}

impl<'s> State<GameData<'s, 's>, MyEvents> for Editor<'_, '_> {
    fn on_start(&mut self, data: StateData<'_, GameData<'s, 's>>) {
        // setup dispatcher
        let mut dispatcher = Editor::create_dispatcher(data.world);
        dispatcher.setup(data.world);
        self.dispatcher = Some(dispatcher);
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
                if self.time_start.elapsed().as_secs() > 0 {
                    return Trans::Pop;
                }
            }
        }

        if let MyEvents::App(event) = &event {
            match event {
                _ => {}
            }
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
            dispatcher.dispatch(&data.world);
        }

        Trans::None
    }
}

impl<'a, 'b> Editor<'a, 'b> {
    fn create_dispatcher(world: &mut World) -> Dispatcher<'a, 'b> {
        let mut dispatcher_builder = DispatcherBuilder::new();
        dispatcher_builder.add(ConsoleInputSystem, "console_input_system", &[]);
        dispatcher_builder.add(
            systems::graphics::PositionDrawUpdateSystem,
            "position_draw_update_system",
            &[],
        );

        dispatcher_builder
            .with_pool((*world.read_resource::<ArcThreadPool>()).clone())
            .build()
    }
}
