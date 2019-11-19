use crate::components::physics::{Cuboid, Velocity};
use crate::systems::physics::CollisionDirection::FromTop;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};

enum CollisionDirection {
    FromTop,
    FromLeft,
    FromBottom,
    FromRight,
}

struct Collider {
    l: f32,
    r: f32,
    u: f32,
    d: f32,
}

#[derive(SystemDesc)]
pub struct CollisionSystem;

impl CollisionSystem {
    fn trans_cuboid_to_collider(trans: &Transform, cuboid: &Cuboid) -> Collider {
        let trans_x = trans.translation().data[0];
        let trans_y = trans.translation().data[1];
        let half_w = cuboid.width / 2.0;
        let half_h = cuboid.height / 2.0;
        Collider {
            l: trans_x - half_w,
            r: trans_x + half_w,
            u: trans_y + half_h,
            d: trans_y - half_h,
        }
    }

    fn collide(
        trans1: &Transform,
        cuboid1: &Cuboid,
        trans2: &Transform,
        cuboid2: &Cuboid,
    ) -> CollisionDirection {
        FromTop
    }
}

impl<'s> System<'s> for CollisionSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        ReadStorage<'s, Cuboid>,
        WriteStorage<'s, Velocity>,
    );

    fn run(&mut self, (mut transforms, tiles, mut velocity): Self::SystemData) {
        for (transform1, cuboid1, velocity1) in (&mut transforms, &tiles, &mut velocity).join() {}
    }
}
