use crate::components::physics::{Collidee, CollideeDetails, CollisionSideOfBlock, Grounded, PlatformCollisionPoints, PlatformCuboid, Position, Velocity, Sticky, GravityDirection};
use crate::pizzatopia::{MAX_FALL_SPEED, MAX_RUN_SPEED, TILE_WIDTH, FRICTION};
use crate::systems::physics::CollisionDirection::FromTop;
use crate::utils::{Vec2, Vec3};
use amethyst::core::{SystemDesc, Transform};
use amethyst::derive::SystemDesc;
use amethyst::ecs::{Join, Read, ReadStorage, System, SystemData, World, WriteStorage, Entity, Entities};
use amethyst::input::{InputHandler, StringBindings};
use log::{debug, error, info, warn};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum CollisionDirection {
    FromTop,
    FromLeft,
    FromBottom,
    FromRight,
}

impl CollisionDirection {
    fn is_horizontal(&self) -> bool {
        match self {
            CollisionDirection::FromLeft => true,
            CollisionDirection::FromRight => true,
            _ => false
        }
    }
}

#[derive(SystemDesc)]
pub struct ApplyStickySystem;

impl<'s> System<'s> for ApplyStickySystem {
    type SystemData = (WriteStorage<'s, Velocity>, ReadStorage<'s, Sticky>, ReadStorage<'s, Collidee>, WriteStorage<'s, GravityDirection>);

    fn run(&mut self, (mut velocities, stickies, collidees, mut gravities): Self::SystemData) {
        for (velocity, sticky, collidee, gravity) in (&mut velocities, &stickies, &collidees, &mut gravities).join() {
            if sticky.0 {
                gravity.0 = match CollisionDirection::is_horizontal(&gravity.0) {
                    true => {match &collidee.vertical {
                        Some(x) => match x.side {
                            CollisionSideOfBlock::Top => CollisionDirection::FromTop,
                            CollisionSideOfBlock::Bottom=> CollisionDirection::FromBottom,
                            _ => gravity.0
                        },
                    None => gravity.0}},
                    false => {match &collidee.horizontal {
                        Some(x) => match x.side {
                            CollisionSideOfBlock::Right => CollisionDirection::FromRight,
                            CollisionSideOfBlock::Left=> CollisionDirection::FromLeft,
                            _ => gravity.0
                        },
                        None => gravity.0}},
                };
            }
        }
    }
}

pub fn gravitationally_adapted_velocity(vel: &Vec2, gravity: &GravityDirection) -> Vec2 {
    match gravity.0 {
        CollisionDirection::FromLeft => {
            Vec2{
                x: -vel.y,
                y: vel.x,
            }
        },
        CollisionDirection::FromRight => {
            Vec2{
                x: vel.y,
                y: -vel.x,
            }
        },
        CollisionDirection::FromTop => {
            Vec2{
                x: vel.x,
                y: vel.y,
            }
        },
        CollisionDirection::FromBottom => {
            Vec2{
                x: -vel.x,
                y: -vel.y,
            }
        },
    }
}

pub fn gravitationally_de_adapted_velocity(vel: &Vec2, gravity: &GravityDirection) -> Vec2 {
    match gravity.0 {
        CollisionDirection::FromLeft => {
            Vec2{
                x: vel.y,
                y: -vel.x,
            }
        },
        CollisionDirection::FromRight => {
            Vec2{
                x: -vel.y,
                y: vel.x,
            }
        },
        CollisionDirection::FromTop => {
            Vec2{
                x: vel.x,
                y: vel.y,
            }
        },
        CollisionDirection::FromBottom => {
            Vec2{
                x: -vel.x,
                y: -vel.y,
            }
        },
    }
}

    #[derive(SystemDesc)]
    pub struct ApplyVelocitySystem;

impl<'s> System<'s> for ApplyVelocitySystem {
    type SystemData = (WriteStorage<'s, Velocity>, WriteStorage<'s, Position>,
    );

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
    type SystemData = (
        WriteStorage<'s, Velocity>,
        ReadStorage<'s, Grounded>,
        ReadStorage<'s, GravityDirection>,
        Read<'s, InputHandler<StringBindings>>,
    );

    fn run(&mut self, (mut velocities, grounded, gravities, input): Self::SystemData) {
        for (velocity, grounded, gravity) in (&mut velocities, (&grounded).maybe(), (&gravities).maybe()).join() {

            let mut grav_dir = CollisionDirection::FromTop;
            if let Some(grav) = gravity {
                grav_dir = grav.0;
            }

            let mut grav_vel = gravitationally_de_adapted_velocity(&velocity.0, &GravityDirection(grav_dir));

            // Apply friction and slow down
            if let Some(ground) = grounded {
                if ground.0 {
                    if let Some(horizontal_movement) = input.axis_value("horizontal_move") {
                        // Not moving or trying to move in opposite direction
                        if horizontal_movement == 0.0 || horizontal_movement * grav_vel.x < 0.0 {
                            if grav_vel.x.abs() <= 0.1 {
                                grav_vel.x = 0.0;
                            } else {
                                // Slow in opposite direction
                                grav_vel.x *= FRICTION;
                            }
                            velocity.0 = gravitationally_adapted_velocity(&grav_vel, &GravityDirection(grav_dir));
                        }
                    }
                }
            }

            let mut gravity_vec = Vec2::new(0.0, -0.28);
            gravity_vec = gravitationally_adapted_velocity(&gravity_vec, &GravityDirection(grav_dir));

            velocity.0.x += gravity_vec.x;
            velocity.0.y += gravity_vec.y;

            // Limit speed
            velocity.0.x = f32::min(velocity.0.x, MAX_RUN_SPEED);
            velocity.0.x = f32::max(velocity.0.x, -MAX_RUN_SPEED);

            velocity.0.y = f32::max(velocity.0.y, -MAX_FALL_SPEED);
        }
    }
}

#[derive(SystemDesc)]
pub struct ActorCollisionSystem;

impl ActorCollisionSystem {
    fn create_corners_with_coll_points_tl_br(pos: &Vec2, points: &PlatformCollisionPoints) -> (Vec2, Vec2) {
        let mut leftmost: f32 = 999999.0;
        let mut rightmost: f32 = -999999.0;
        let mut topmost: f32 = -999999.0;
        let mut bottommost: f32 = 999999.0;
        // Go through every collision point to create cuboid shape
        for collider_offset in &points.0 {
            let point_pos = Vec2::new(
                collider_offset.x + pos.x,
                collider_offset.y + pos.y,
            );

            leftmost = leftmost.min(point_pos.x);
            bottommost = bottommost.min(point_pos.y);
            rightmost = rightmost.max(point_pos.x);
            topmost = topmost.max(point_pos.y);
        }
        //info!("TopLeft: {:?}, BottomRight: {:?}", Vec2::new(leftmost, topmost), Vec2::new(rightmost, bottommost));
        (Vec2::new(leftmost, topmost), Vec2::new(rightmost, bottommost))
    }

    fn cuboid_intersection(top_left1: &Vec2, bottom_right1: &Vec2, top_left2: &Vec2, bottom_right2: &Vec2) -> bool {
        // Check if the x axis of both squares ever intersect
        let halfway_point = bottom_right2.x + (top_left1.x - bottom_right2.x) / 2.0;
        //info!("Halfway point X: {:?}", halfway_point);
        if !(halfway_point > top_left1.x && halfway_point < bottom_right1.x &&
            halfway_point > top_left2.x && halfway_point < bottom_right2.x) {
            // no intersects
            return false;
        }

        // Check if the y axis of both squares ever intersect
        let halfway_point = top_left2.y + (bottom_right1.y - top_left2.y) / 2.0;
        //info!("Halfway point Y: {:?}", halfway_point);
        if !(halfway_point < top_left1.y && halfway_point > bottom_right1.y &&
            halfway_point < top_left2.y && halfway_point > bottom_right2.y) {
            // no intersects
            return false;
        }

        true
    }
}

impl<'s> System<'s> for ActorCollisionSystem {
    type SystemData = (
        ReadStorage<'s, Position>,
        ReadStorage<'s, PlatformCollisionPoints>,
        Entities<'s>,
    );

    fn run(&mut self, (positions, coll_points, entities): Self::SystemData) {
        for (ent_pos1, coll_point1, entity1) in (&positions, &coll_points, &entities).join() {
            let pos1 = Vec2::new(ent_pos1.0.x, ent_pos1.0.y);
            let (top_left1, bottom_right1) = Self::create_corners_with_coll_points_tl_br(&pos1, coll_point1);

            for (ent_pos2, coll_point2, entity2) in (&positions, &coll_points, &entities).join() {
                if entity1 == entity2 {
                    continue;
                }

                let pos2 = Vec2::new(ent_pos2.0.x, ent_pos2.0.y);
                let (top_left2, bottom_right2) = Self::create_corners_with_coll_points_tl_br(&pos2, coll_point2);
                if Self::cuboid_intersection(&top_left1, &bottom_right1, &top_left2, &bottom_right2) {
                    warn!("Wow! A collision! {}", pos1.x);
                }
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
        cuboid_pos: &Vec2,
        cuboid: &PlatformCuboid,
    ) -> Option<(Vec2, CollisionSideOfBlock)> {
        let x_intersects = Self::intersect_x(point, cuboid_pos, cuboid);
        let y_intersects = Self::intersect_y(point, cuboid_pos, cuboid);

        // The point must be outside the tile
        if x_intersects && y_intersects {
            debug!("Already inside block!");
            return None;
        }

        // Find out which sides of the block we want to use
        let hor_side = match vel.x > 0.0 {
            true => cuboid_pos.x - cuboid.half_width,
            false => cuboid_pos.x + cuboid.half_width,
        };
        let ver_side = match vel.y > 0.0 {
            true => cuboid_pos.y - cuboid.half_height,
            false => cuboid_pos.y + cuboid.half_height,
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
                        point, cuboid_pos
                    );
                    return None;
                }
            },
        };
        debug!("Point: {:?}, Block: {:?}", point, cuboid_pos);
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
                        point, cuboid_pos
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

        match Self::intersect_x(&result, &cuboid_pos, cuboid)
            && Self::intersect_y(&result, &cuboid_pos, cuboid)
        {
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
        ReadStorage<'s, GravityDirection>,
    );

    fn run(
        &mut self,
        (mut velocities, mut collidees, positions, cuboids, coll_points, gravities): Self::SystemData,
    ) {
        for (velocity, collidee, ent_pos, coll_point, gravity) in
            (&mut velocities, &mut collidees, &positions, &coll_points, (&gravities).maybe()).join()
        {
            // Reset collidees here they can be used for the rest of the frame
            collidee.horizontal = None;
            collidee.vertical = None;
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
                        let delta = TILE_WIDTH;
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
        WriteStorage<'s, Grounded>,
        ReadStorage<'s, GravityDirection>,
    );

    fn run(
        &mut self,
        (mut velocities, mut positions, mut collidees, mut grounded, gravities): Self::SystemData,
    ) {
        for (velocity, position, collidee, mut grounded, gravity) in (
            &mut velocities,
            &mut positions,
            &mut collidees,
            (&mut grounded).maybe(),
            (&gravities).maybe(),
        )
            .join()
        {
            // not grounded by default, grounded if found
            if let Some(ground) = &mut grounded {
                ground.0 = false;
            };

            let mut grav_dir = CollisionDirection::FromTop;
            if let Some(grav) = gravity {
                grav_dir = grav.0;
            }
            if let Some(cdee) = &collidee.vertical {
                match cdee.side {
                    CollisionSideOfBlock::Bottom => {
                        position.0.y += cdee.correction;
                        if let Some(ground) = &mut grounded {
                            ground.0 = grav_dir == CollisionDirection::FromBottom;
                        };
                    }
                    CollisionSideOfBlock::Top => {
                        position.0.y += cdee.correction;
                        if let Some(ground) = &mut grounded {
                            ground.0 = grav_dir == CollisionDirection::FromTop;
                        };
                    }
                    _ => {
                    }
                }
                velocity.0.y = 0.0;
            }

            if let Some(cdee) = &collidee.horizontal {
                match cdee.side {
                    CollisionSideOfBlock::Left => {
                        if let Some(ground) = &mut grounded {
                            ground.0 = grav_dir == CollisionDirection::FromLeft;
                        };
                    }
                    CollisionSideOfBlock::Right => {
                        if let Some(ground) = &mut grounded {
                            ground.0 = grav_dir == CollisionDirection::FromRight;
                        };
                    }
                    _ => {
                    }
                }
                position.0.x += cdee.correction;
                velocity.0.x = 0.0;
            }

            //collidee.horizontal = None;
            //collidee.vertical = None;
        }
    }
}
