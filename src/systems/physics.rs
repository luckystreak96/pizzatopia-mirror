use crate::components::physics::{PlatformCollisionPoints, PlatformCuboid, Position, Velocity};
use crate::systems::physics::CollisionDirection::FromTop;
use crate::utils::Vec2;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};

pub(crate) enum CollisionDirection {
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
    fn trans_cuboid_to_collider(trans: &Transform, cuboid: &PlatformCuboid) -> Collider {
        let trans_x = trans.translation().data[0];
        let trans_y = trans.translation().data[1];
        let half_w = cuboid.half_width / 2.0;
        let half_h = cuboid.half_height / 2.0;
        Collider {
            l: trans_x - half_w,
            r: trans_x + half_w,
            u: trans_y + half_h,
            d: trans_y - half_h,
        }
    }

    fn collide(
        trans1: &Transform,
        cuboid1: &PlatformCuboid,
        trans2: &Transform,
        cuboid2: &PlatformCuboid,
    ) -> CollisionDirection {
        FromTop
    }
}

impl<'s> System<'s> for CollisionSystem {
    type SystemData = (
        WriteStorage<'s, Transform>,
        ReadStorage<'s, PlatformCuboid>,
        WriteStorage<'s, Velocity>,
    );

    fn run(&mut self, (mut transforms, tiles, mut velocity): Self::SystemData) {
        for (transform1, cuboid1, velocity1) in (&mut transforms, &tiles, &mut velocity).join() {}
    }
}

#[derive(SystemDesc)]
pub struct ApplyVelocitySystem;

impl<'s> System<'s> for ApplyVelocitySystem {
    type SystemData = (WriteStorage<'s, Velocity>, WriteStorage<'s, Position>);

    fn run(&mut self, (mut velocities, mut positions): Self::SystemData) {
        for (velocity, position) in (&mut velocities, &mut positions).join() {
            position.0.x += velocity.0.x;
            position.0.y += velocity.0.y;
            velocity.0.x = 0.0;
            velocity.0.y = 0.0;
        }
    }
}

#[derive(SystemDesc)]
pub struct PlatformCollisionSystem;

impl<'s> System<'s> for PlatformCollisionSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, PlatformCuboid>,
        ReadStorage<'s, PlatformCollisionPoints>,
    );

    fn run(&mut self, (mut velocities, positions, cuboids, coll_points): Self::SystemData) {
        for (velocity, ent_pos, coll_point) in (&mut velocities, &positions, &coll_points).join() {
            for (plat_pos, cuboid) in (&positions, &cuboids).join() {
                for point in &coll_point.0 {
                    let point_pos = Vec2::new(
                        point.x + ent_pos.0.x + velocity.0.x,
                        point.y + ent_pos.0.y + velocity.0.y,
                    );

                    // Horizontal intersect
                    if point_pos.x <= plat_pos.0.x + cuboid.half_width
                        && point_pos.x >= plat_pos.0.x - cuboid.half_width
                    {
                        // Vertical intersect
                        if point_pos.y <= plat_pos.0.y + cuboid.half_height
                            && point_pos.y >= plat_pos.0.y - cuboid.half_height
                        {
                            // There is intersection
                            velocity.0.x = 0.0;
                            velocity.0.y = 0.0;
                        }
                    }
                }
            }
        }
    }
}
