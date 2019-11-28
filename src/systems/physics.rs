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
use log::{debug, error, info, warn};

pub(crate) enum CollisionDirection {
    FromTop,
    FromLeft,
    FromBottom,
    FromRight,
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
    fn raycast(
        point: &Vec2,
        vel: &Vec2,
        pos: &Vec2,
        cuboid: &PlatformCuboid,
    ) -> Option<(Vec2, CollisionSideOfBlock)> {
        let x_intersects = Self::intersect_x(point, pos, cuboid);
        let y_intersects = Self::intersect_y(point, pos, cuboid);

        // The point must be outside the tile
        if x_intersects && y_intersects {
            debug!("Already inside block!");
            return None;
        }

        // Find out which sides of the block we want to use
        let hor_side = match vel.x > 0.0 {
            true => pos.x - cuboid.half_width,
            false => pos.x + cuboid.half_width,
        };
        let ver_side = match vel.y > 0.0 {
            true => pos.y - cuboid.half_height,
            false => pos.y + cuboid.half_height,
        };

        let hor_dist = hor_side - point.x;
        let ver_dist = ver_side - point.y;

        let magic_number = -9999.0;

        let mut vertical_perc_to_collision = match vel.y != 0.0 {
            true => ver_dist / vel.y,
            false => match y_intersects {
                true => magic_number,
                false => {
                    debug!(
                        "Division by 0 and no y-collision! Point: {:?}, Block: {:?}",
                        point, pos
                    );
                    return None;
                }
            },
        };
        debug!("Point: {:?}, Block: {:?}", point, pos);
        debug!(
            "Percentage calc vertical: distance({}) / vel({})",
            ver_dist, vel.y
        );

        let mut horizontal_perc_to_collision = match vel.x != 0.0 {
            true => hor_dist / vel.x,
            false => match x_intersects {
                true => magic_number,
                false => {
                    debug!(
                        "Division by 0 and no x-collision! Point: {:?}, Block: {:?}",
                        point, pos
                    );
                    return None;
                }
            },
        };

        debug!(
            "Percentage calc horizontal: distance({}) / vel({})",
            hor_dist, vel.x
        );

        // The block is farther away than where we go
        if horizontal_perc_to_collision > 1.0 {
            horizontal_perc_to_collision = magic_number;
        }
        if vertical_perc_to_collision > 1.0 {
            vertical_perc_to_collision = magic_number;
        }

        // Doesn't collide at all - need to go in the opposite direction
        if horizontal_perc_to_collision < 0.0
            && horizontal_perc_to_collision != magic_number
            && !x_intersects
            || vertical_perc_to_collision < 0.0
                && vertical_perc_to_collision != magic_number
                && !y_intersects
        {
            return None;
        }

        // There's no collision at all, they both go to infinity
        if horizontal_perc_to_collision == magic_number
            && vertical_perc_to_collision == magic_number
        {
            return None;
        }

        // If neither directions are legit, there's no collision
        if horizontal_perc_to_collision >= 0.0 && horizontal_perc_to_collision <= 1.0
            || vertical_perc_to_collision <= 1.0 && vertical_perc_to_collision >= 0.0
        {
        } else {
            return None;
        }

        let side = match horizontal_perc_to_collision > vertical_perc_to_collision {
            true => match vel.x > 0.0 {
                true => CollisionSideOfBlock::Left,
                false => CollisionSideOfBlock::Right,
            },
            false => match vel.y > 0.0 {
                true => CollisionSideOfBlock::Bottom,
                false => CollisionSideOfBlock::Top,
            },
        };

        let result = match side.is_horizontal() {
            true => Vec2::new(hor_side, point.y + horizontal_perc_to_collision * vel.y),
            false => Vec2::new(point.x + vertical_perc_to_collision * vel.x, ver_side),
        };

        debug!(
            "Percentages = x:{}, y:{}",
            horizontal_perc_to_collision, vertical_perc_to_collision
        );
        debug!("Values = {:?}", result);

        match Self::intersect_x(&result, &pos, cuboid) && Self::intersect_y(&result, &pos, cuboid) {
            true => Some((result, side)),
            false => None,
        }
    }

    fn intersect_x(point: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid) -> bool {
        Self::within_range_x(point, pos, cuboid, 0.0)
    }

    fn intersect_y(point: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid) -> bool {
        Self::within_range_y(point, pos, cuboid, 0.0)
    }

    fn within_range_x(point: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid, delta: f32) -> bool {
        if point.x <= pos.x + cuboid.half_width + delta
            && point.x >= pos.x - cuboid.half_width - delta
        {
            return true;
        }
        return false;
    }

    fn within_range_y(point: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid, delta: f32) -> bool {
        if point.y <= pos.y + cuboid.half_height + delta
            && point.y >= pos.y - cuboid.half_height - delta
        {
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
            // We want to loop up to twice here
            // First loop finds the closest collision of all the points
            // Second loop tries to find a collision in the other axis
            //  given the changes in position and velocity
            // These bad boys get modified at the end of the loop
            let mut current_vel = velocity.0.clone();
            let mut current_ent_pos = ent_pos.0.clone();
            loop {
                debug!("Velocity: {:?}", current_vel);
                // We want the shortest distance collision of all points
                let mut prev_distance_to_collision = 9999.0;
                let mut details = None;

                // Go through every collision point
                for collider_offset in &coll_point.0 {
                    let point_pos = Vec2::new(
                        collider_offset.x + current_ent_pos.x,
                        collider_offset.y + current_ent_pos.y,
                    );
                    debug!("Point: {:?}", point_pos);

                    for (plat_pos, cuboid) in (&positions, &cuboids).join() {
                        // delta so if we go through the corner of a block we still check
                        // Must stay gt or eq to the max speed that will be reached
                        let delta = 10.0;
                        let platform_position = plat_pos.0.clone();
                        let point_vel_pos = Vec2::new(
                            collider_offset.x + ent_pos.0.x + current_vel.x,
                            collider_offset.y + ent_pos.0.y + current_vel.y,
                        );

                        // Is the block even close to us
                        if Self::within_range_x(&point_vel_pos, &plat_pos.0, cuboid, delta) {
                            if Self::within_range_y(&point_vel_pos, &plat_pos.0, cuboid, delta) {
                                // point of collision and side
                                let point_of_collision = Self::raycast(
                                    &point_pos,
                                    &current_vel,
                                    &platform_position,
                                    cuboid,
                                );

                                // skip if no collision
                                if point_of_collision.is_none() {
                                    continue;
                                }
                                let (mut point_of_collision, side) = point_of_collision.unwrap();

                                // find the distance to the point to see if another collision was closer
                                let distance_to_point_vec =
                                    Vec2::subtract(&point_pos, &point_of_collision);
                                let cur_distance_to_collision = (distance_to_point_vec.x.powi(2)
                                    + distance_to_point_vec.y.powi(2))
                                .sqrt();

                                if cur_distance_to_collision > prev_distance_to_collision {
                                    continue;
                                }
                                prev_distance_to_collision = cur_distance_to_collision;

                                // Calculate the vertical/horizontal correction to be applied
                                let mut correction = 0.01;
                                let mut new_velocity =
                                    Vec2::subtract(&current_vel, &distance_to_point_vec);
                                match side.clone() {
                                    CollisionSideOfBlock::Top => {
                                        new_velocity.y = 0.0;
                                        point_of_collision.y += correction;
                                        correction = point_of_collision.y - point_pos.y;
                                    }
                                    CollisionSideOfBlock::Bottom => {
                                        new_velocity.y = 0.0;
                                        point_of_collision.y -= correction;
                                        correction = point_of_collision.y - point_pos.y;
                                    }
                                    CollisionSideOfBlock::Left => {
                                        new_velocity.x = 0.0;
                                        point_of_collision.x -= correction;
                                        correction = point_of_collision.x - point_pos.x;
                                    }
                                    CollisionSideOfBlock::Right => {
                                        new_velocity.x = 0.0;
                                        point_of_collision.x += correction;
                                        correction =
                                            point_of_collision.x + correction - point_pos.x;
                                    }
                                };
                                debug!("Side: {:?}", side.clone());
                                debug!("Correction: {}", correction);

                                details = Some(CollideeDetails {
                                    name: String::from("Some block"),
                                    position: platform_position,
                                    half_size: Vec2::new(cuboid.half_width, cuboid.half_height),
                                    correction,
                                    distance: cur_distance_to_collision,
                                    new_collider_pos: Vec2::subtract(
                                        &point_of_collision,
                                        &collider_offset,
                                    ),
                                    new_collider_vel: new_velocity,
                                    side: side.clone(),
                                });
                            }
                        }
                    }
                }

                match details {
                    Some(det) => {
                        debug!("Collided on side: {:?}", det.side);
                        current_ent_pos = det.new_collider_pos.clone();
                        current_vel = det.new_collider_vel.clone();
                        match det.side.is_horizontal() {
                            true => collidee.horizontal = Some(det),
                            false => collidee.vertical = Some(det),
                        };
                        // Both collisions are found -> we done
                        if collidee.both() {
                            break;
                        }
                    }
                    // No collision -> we done
                    None => {
                        break;
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

            //info!("Position: {:?}", position.0);

            collidee.horizontal = None;
            collidee.vertical = None;
        }
    }
}
