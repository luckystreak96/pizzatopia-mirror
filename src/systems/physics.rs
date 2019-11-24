use crate::components::physics::{
    Collidee, CollideeDetails, CollisionSideOfBlock, PlatformCollisionPoints, PlatformCuboid,
    Position, Velocity,
};
use crate::pizzatopia::MAX_FALL_SPEED;
use crate::systems::physics::CollisionDirection::FromTop;
use crate::utils::Vec2;
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage};
use log::info;

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
        }
    }
}

#[derive(SystemDesc)]
pub struct ApplyGravitySystem;

impl<'s> System<'s> for ApplyGravitySystem {
    type SystemData = (WriteStorage<'s, Velocity>);

    fn run(&mut self, (mut velocities): Self::SystemData) {
        for (velocity) in (&mut velocities).join() {
            velocity.0.y -= 0.16;
            if velocity.0.y < -MAX_FALL_SPEED {
                velocity.0.y = -MAX_FALL_SPEED;
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct PlatformCollisionSystem;

impl PlatformCollisionSystem {
    fn intersect_x(point: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid) -> bool {
        if point.x < pos.x + cuboid.half_width && point.x > pos.x - cuboid.half_width {
            return true;
        }
        return false;
    }

    fn intersect_y(point: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid) -> bool {
        if point.y < pos.y + cuboid.half_height && point.y > pos.y - cuboid.half_height {
            return true;
        }
        return false;
    }
}

impl<'s> System<'s> for PlatformCollisionSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Collidee>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, PlatformCuboid>,
        ReadStorage<'s, PlatformCollisionPoints>,
    );

    fn run(
        &mut self,
        (mut velocities, mut collidees, positions, cuboids, coll_points): Self::SystemData,
    ) {
        for (velocity, collidee, ent_pos, coll_point) in
            (&mut velocities, &mut collidees, &positions, &coll_points).join()
        {
            for (plat_pos, cuboid) in (&positions, &cuboids).join() {
                for point in &coll_point.0 {
                    let point_pos = Vec2::new(
                        point.x + ent_pos.0.x + velocity.0.x,
                        point.y + ent_pos.0.y + velocity.0.y,
                    );

                    // Horizontal intersect
                    if Self::intersect_x(&point_pos, &plat_pos.0, cuboid) {
                        // Vertical intersect
                        if Self::intersect_y(&point_pos, &plat_pos.0, cuboid) {
                            let name = String::from("Some block");
                            let position = Vec2::new(plat_pos.0.x, plat_pos.0.y);
                            let half_size = Vec2::new(cuboid.half_width, cuboid.half_height);

                            let mut side = None;

                            let iterations = 4;
                            let tmp_vel = Vec2::new(
                                velocity.0.x / iterations as f32,
                                velocity.0.y / iterations as f32,
                            );

                            // Split the velocity by 4 - calculate 4 dummy frames till 1 collision is possible but not the other
                            for i in 0..iterations {
                                let vel_x_frame =
                                    velocity.0.x - i as f32 * velocity.0.x / iterations as f32;
                                let vel_y_frame =
                                    velocity.0.y - i as f32 * velocity.0.y / iterations as f32;
                                let tmp_point =
                                    Vec2::new(point.x + vel_x_frame, point.y + vel_y_frame);

                                // Is there only 1 possible collision?
                                // Vertical intersect
                                if Self::intersect_x(&tmp_point, &plat_pos.0, cuboid) {
                                    side = match velocity.0.y > 0.0 {
                                        true => Some(CollisionSideOfBlock::Bottom),
                                        false => Some(CollisionSideOfBlock::Top),
                                    };
                                    break;
                                }
                                // Horizontal intersect
                                else if Self::intersect_y(&tmp_point, &plat_pos.0, cuboid) {
                                    side = match velocity.0.x > 0.0 {
                                        true => Some(CollisionSideOfBlock::Left),
                                        false => Some(CollisionSideOfBlock::Right),
                                    };
                                    break;
                                }
                            }

                            if side.is_none() {
                                match ent_pos.0.y > plat_pos.0.y {
                                    true => side = Some(CollisionSideOfBlock::Top),
                                    false => side = Some(CollisionSideOfBlock::Bottom),
                                };
                            }

                            let side = side.unwrap();

                            // Calculate the vertical/horizontal correction to be applied
                            let mut correction = 0.0;
                            match side {
                                CollisionSideOfBlock::Top => {
                                    correction =
                                        (position.y + half_size.y) - (point.y + ent_pos.0.y)
                                }
                                CollisionSideOfBlock::Bottom => {
                                    correction =
                                        (position.y + half_size.y) - (point.y + ent_pos.0.y)
                                }
                                CollisionSideOfBlock::Left => {
                                    correction =
                                        (position.x + half_size.x) - (point.x + ent_pos.0.x)
                                }
                                CollisionSideOfBlock::Right => {
                                    correction =
                                        (position.x + half_size.x) - (point.x + ent_pos.0.x)
                                }
                            };

                            let details = CollideeDetails {
                                name,
                                position,
                                half_size,
                                correction,
                                side: side.clone(),
                            };

                            match side {
                                CollisionSideOfBlock::Right => {
                                    collidee.horizontal = Some(details);
                                }
                                CollisionSideOfBlock::Left => {
                                    collidee.horizontal = Some(details);
                                }
                                _ => {
                                    collidee.vertical = Some(details);
                                }
                            };
                        }
                    }
                }
            }
        }
    }
}

#[derive(SystemDesc)]
pub struct ApplyCollisionSystem;

impl<'s> System<'s> for ApplyCollisionSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Position>,
        WriteStorage<'s, Collidee>,
    );

    fn run(&mut self, (mut velocities, mut positions, mut collidees): Self::SystemData) {
        for (velocity, position, collidee) in
            (&mut velocities, &mut positions, &mut collidees).join()
        {
            if let Some(cdee) = &collidee.vertical {
                match cdee.side {
                    CollisionSideOfBlock::Bottom => {
                        position.0.y += cdee.correction;
                    }
                    CollisionSideOfBlock::Top => {
                        position.0.y += cdee.correction;
                        // on_ground logic
                    }
                    _ => {}
                };
                velocity.0.y = 0.0;
            }

            if let Some(cdee) = &collidee.horizontal {
                position.0.x += cdee.correction;
                velocity.0.x = 0.0;
            }

            collidee.horizontal = None;
            collidee.vertical = None;
        }
    }
}
