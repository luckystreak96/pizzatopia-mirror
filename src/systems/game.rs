use crate::components::game::{CollisionEvent, Health, Invincibility};
use crate::components::graphics::AnimationCounter;
use crate::components::physics::{GravityDirection, PlatformCuboid, Position, Velocity};
use crate::pizzatopia::{TILE_HEIGHT, TILE_WIDTH};
use crate::systems::physics::{gravitationally_de_adapted_velocity, CollisionDirection};
use amethyst::core::math::Vector3;
use amethyst::core::shrev::{EventChannel, ReaderId};
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Entities, Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use amethyst::renderer::{
    Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture,
};
use log::warn;

pub const IFRAMES_PER_HIT: u32 = 90;

#[derive(SystemDesc)]
#[system_desc(name(EnemyCollisionSystemDesc))]
pub struct EnemyCollisionSystem {
    #[system_desc(event_channel_reader)]
    reader: ReaderId<CollisionEvent>,
}

impl EnemyCollisionSystem {
    pub(crate) fn new(reader: ReaderId<CollisionEvent>) -> Self {
        Self { reader }
    }
}

impl<'s> System<'s> for EnemyCollisionSystem {
    type SystemData = (
        WriteStorage<'s, Health>,
        WriteStorage<'s, Invincibility>,
        Entities<'s>,
        Read<'s, EventChannel<CollisionEvent>>,
    );

    fn run(
        &mut self,
        (mut healths, mut invincibilities, entities, event_channel): Self::SystemData,
    ) {
        for event in event_channel.read(&mut self.reader) {
            match event {
                CollisionEvent::EnemyCollision(entity_id, damage) => {
                    let iframes = &mut invincibilities
                        .get_mut(entities.entity(*entity_id))
                        .expect("Tried to hurt entity with no iframes component")
                        .0;
                    let health = &mut healths
                        .get_mut(entities.entity(*entity_id))
                        .expect("Tried to hurt entity with no health component")
                        .0;
                    if *health > 0 && *iframes == 0 {
                        *health -= *damage;
                        *iframes += IFRAMES_PER_HIT;
                    }
                    warn!("Health is now {}", health);
                }
            }
            //println!("Received an event: {:?}", event);
        }
    }
}

#[derive(SystemDesc)]
pub struct InvincibilitySystem;

impl<'s> System<'s> for InvincibilitySystem {
    type SystemData = (WriteStorage<'s, Invincibility>, Entities<'s>);

    fn run(&mut self, (mut invincibilities, entities): Self::SystemData) {
        for (mut invinc, entity) in (&mut invincibilities, &entities).join() {
            if invinc.0 > 0 {
                invinc.0 -= 1;
            }
        }
    }
}