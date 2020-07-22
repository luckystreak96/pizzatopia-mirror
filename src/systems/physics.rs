use crate::{
    components::physics::{
        Collidee, CollideeDetails, CollisionPoint, CollisionSideOfBlock, Ducking, GravityDirection,
        Grounded, PlatformCollisionPoints, PlatformCuboid, Position, RTreeEntity, Sticky, Velocity,
    },
    events::Events,
    states::pizzatopia::{FRICTION, MAX_FALL_SPEED, MAX_RUN_SPEED, TILE_WIDTH},
    systems::physics::CollisionDirection::FromTop,
    utils::{Vec2, Vec3},
};
use amethyst::{
    core::Transform,
    ecs::{Entities, Entity},
};
use log::debug;

use crate::components::game::Block;
use crate::components::physics::ChildTo;
use crate::{
    components::game::{CollisionEvent, Damage, Player, Projectile, Reflect, Team},
    systems::input::InputManager,
};
use amethyst::{
    core::{
        bundle::SystemBundle,
        frame_limiter::FrameRateLimitStrategy,
        shrev::{EventChannel, ReaderId},
        timing::Time,
        SystemDesc,
    },
    derive::SystemDesc,
    ecs::{
        Component, DenseVecStorage, Join, Read, ReadStorage, System, SystemData, World, Write,
        WriteStorage,
    },
    input::{InputHandler, StringBindings},
};
use num_traits::identities::Zero;
use rstar::{RTree, AABB};
use std::collections::HashSet;

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
            _ => false,
        }
    }
}

#[derive(SystemDesc)]
pub struct ApplyStickySystem;

impl<'s> System<'s> for ApplyStickySystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        ReadStorage<'s, Sticky>,
        ReadStorage<'s, Collidee>,
        WriteStorage<'s, GravityDirection>,
    );

    fn run(&mut self, (mut velocities, stickies, collidees, mut gravities): Self::SystemData) {
        for (velocity, sticky, collidee, gravity) in
            (&mut velocities, &stickies, &collidees, &mut gravities).join()
        {
            if sticky.0 {
                let prev_gravity = gravity.0;
                let mut collidee_velocity = velocity.vel.clone();
                gravity.0 = match CollisionDirection::is_horizontal(&gravity.0) {
                    true => match &collidee.vertical {
                        Some(x) => {
                            collidee_velocity = x.old_collider_vel.clone();
                            match x.side {
                                CollisionSideOfBlock::Top => CollisionDirection::FromTop,
                                CollisionSideOfBlock::Bottom => CollisionDirection::FromBottom,
                                _ => gravity.0,
                            }
                        }
                        None => gravity.0,
                    },
                    false => match &collidee.horizontal {
                        Some(x) => {
                            collidee_velocity = x.old_collider_vel.clone();
                            match x.side {
                                CollisionSideOfBlock::Right => CollisionDirection::FromRight,
                                CollisionSideOfBlock::Left => CollisionDirection::FromLeft,
                                _ => gravity.0,
                            }
                        }
                        None => gravity.0,
                    },
                };

                // Check for walking off platform -> need to change gravity + adapt velocity
                let lean_off_ledge = collidee.current_collision_points() == 0
                    && collidee.prev_collision_points() == 1;
                let is_following_gravity = match gravity.0 {
                    CollisionDirection::FromTop => velocity.vel.y < 0.0,
                    CollisionDirection::FromBottom => velocity.vel.y > 0.0,
                    CollisionDirection::FromLeft => velocity.vel.x > 0.0,
                    CollisionDirection::FromRight => velocity.vel.x < 0.0,
                };
                if lean_off_ledge && is_following_gravity {
                    // Change gravity
                    gravity.0 = match gravity.0 {
                        CollisionDirection::FromTop | CollisionDirection::FromBottom => {
                            if velocity.vel.x > 0.0 {
                                CollisionDirection::FromRight
                            } else {
                                CollisionDirection::FromLeft
                            }
                        }
                        CollisionDirection::FromLeft | CollisionDirection::FromRight => {
                            if velocity.vel.y > 0.0 {
                                CollisionDirection::FromTop
                            } else {
                                CollisionDirection::FromBottom
                            }
                        }
                    }
                }

                if collidee.current_collision_points() == 0 && collidee.prev_collision_points() != 0
                {
                }

                if prev_gravity != gravity.0 {
                    // Only keep movement momentum if not jumping off platform
                    if collidee_velocity.x == 0.0 || collidee_velocity.y == 0.0 || lean_off_ledge {
                        velocity.vel =
                            adapt_sticky_velocity(&collidee_velocity, &prev_gravity, &gravity.0);
                        if lean_off_ledge {
                            match gravity.0 {
                                CollisionDirection::FromBottom => velocity.vel.y = TILE_WIDTH / 4.0,
                                CollisionDirection::FromTop => velocity.vel.y = -TILE_WIDTH / 4.0,
                                CollisionDirection::FromLeft => velocity.vel.x = TILE_WIDTH / 4.0,
                                CollisionDirection::FromRight => velocity.vel.x = -TILE_WIDTH / 4.0,
                            }
                        }
                        //                        println!("Changed vel from {:?} to {:?}", collidee_velocity, velocity.vel);
                    }
                }
            }
        }
    }
}

pub fn adapt_sticky_velocity(
    vel: &Vec2,
    prev_gravity: &CollisionDirection,
    cur_gravity: &CollisionDirection,
) -> Vec2 {
    match prev_gravity {
        CollisionDirection::FromRight => match cur_gravity {
            CollisionDirection::FromTop => Vec2 { x: -vel.y, y: 0.0 },
            CollisionDirection::FromBottom => Vec2 { x: vel.y, y: 0.0 },
            _ => vel.clone(),
        },
        CollisionDirection::FromLeft => match cur_gravity {
            CollisionDirection::FromTop => Vec2 { x: vel.y, y: 0.0 },
            CollisionDirection::FromBottom => Vec2 { x: -vel.y, y: 0.0 },
            _ => vel.clone(),
        },
        CollisionDirection::FromTop => match cur_gravity {
            CollisionDirection::FromRight => Vec2 { x: 0.0, y: -vel.x },
            CollisionDirection::FromLeft => Vec2 { x: 0.0, y: vel.x },
            _ => vel.clone(),
        },
        CollisionDirection::FromBottom => match cur_gravity {
            CollisionDirection::FromRight => Vec2 { x: 0.0, y: vel.x },
            CollisionDirection::FromLeft => Vec2 { x: 0.0, y: -vel.x },
            _ => vel.clone(),
        },
    }
}

pub fn gravitationally_adapted_velocity(vel: &Vec2, gravity: &GravityDirection) -> Vec2 {
    match gravity.0 {
        CollisionDirection::FromLeft => Vec2 {
            x: -vel.y,
            y: vel.x,
        },
        CollisionDirection::FromRight => Vec2 {
            x: vel.y,
            y: -vel.x,
        },
        CollisionDirection::FromTop => Vec2 { x: vel.x, y: vel.y },
        CollisionDirection::FromBottom => Vec2 {
            x: -vel.x,
            y: -vel.y,
        },
    }
}

pub fn gravitationally_de_adapted_velocity(vel: &Vec2, gravity: &GravityDirection) -> Vec2 {
    match gravity.0 {
        CollisionDirection::FromLeft => Vec2 {
            x: vel.y,
            y: -vel.x,
        },
        CollisionDirection::FromRight => Vec2 {
            x: -vel.y,
            y: vel.x,
        },
        CollisionDirection::FromTop => Vec2 { x: vel.x, y: vel.y },
        CollisionDirection::FromBottom => Vec2 {
            x: -vel.x,
            y: -vel.y,
        },
    }
}

#[derive(SystemDesc)]
pub struct ApplyVelocitySystem;

impl<'s> System<'s> for ApplyVelocitySystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Position>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut velocities, mut positions, time): Self::SystemData) {
        for (velocity, position) in (&mut velocities, &mut positions).join() {
            let projection = velocity.project_move(time.time_scale());
            position.0.x += projection.x;
            position.0.y += projection.y;
            // if !velocity.vel.x.is_zero() {
            //     velocity.prev_going_right = velocity.vel.x.is_sign_positive();
            // }
        }
    }
}

#[derive(SystemDesc)]
pub struct ChildPositionSystem;

impl<'s> System<'s> for ChildPositionSystem {
    type SystemData = (
        ReadStorage<'s, Velocity>,
        ReadStorage<'s, ChildTo>,
        WriteStorage<'s, Position>,
        ReadStorage<'s, Ducking>,
        ReadStorage<'s, Block>,
        Entities<'s>,
    );

    fn run(
        &mut self,
        (velocities, children, mut positions, duckings, blockers, entities): Self::SystemData,
    ) {
        for (child, entity) in (&children, &entities).join() {
            let going_right = velocities
                .get(child.parent)
                .unwrap_or(&Velocity::default())
                .prev_going_right;
            let ducking = duckings.get(child.parent).is_some();
            let blocking = blockers.get(entity).is_some();
            let parent_pos = positions
                .get(child.parent)
                .unwrap_or(&Position(Vec3::default()))
                .0
                .to_vec2();

            let position = positions.get_mut(entity).unwrap();
            let offset_x = child.offset.x
                * match going_right {
                    true => 1.,
                    false => -1.,
                };
            let offset_y = {
                match blocking {
                    true => {
                        child.offset.y
                            * match ducking {
                                true => -1.,
                                false => 1.,
                            }
                    }
                    false => child.offset.y,
                }
            };
            position.0.x = offset_x + parent_pos.x;
            position.0.y = offset_y + parent_pos.y;
        }
    }
}

pub struct DuckTransferSystem;

impl<'s> System<'s> for DuckTransferSystem {
    type SystemData = (
        WriteStorage<'s, Ducking>,
        WriteStorage<'s, PlatformCollisionPoints>,
        Entities<'s>,
        Read<'s, InputManager>,
        ReadStorage<'s, Player>,
        ReadStorage<'s, Grounded>,
        ReadStorage<'s, Position>,
        Read<'s, RTree<RTreeEntity>>,
    );

    fn run(
        &mut self,
        (mut duckings, mut collisions, entities, input, players, groundeds, positions, rtree): Self::SystemData,
    ) {
        let threshold = -0.25;
        let mut to_remove = Vec::new();
        for (_duck, points, entity, position) in
            (&mut duckings, &mut collisions, &entities, &positions).join()
        {
            if input.action_status("vertical").axis >= threshold {
                let bottom_left = [
                    position.0.x - points.half_size.x,
                    position.0.y - points.half_size.y,
                ];
                let top_right = [
                    position.0.x + points.half_size.x,
                    position.0.y + points.half_size.y,
                ];
                let intersections = rtree
                    .locate_in_envelope_intersecting(&AABB::from_corners(bottom_left, top_right));
                if intersections.count() == 0 {
                    points.reset_collision_points();
                    to_remove.push(entity);
                }
            }
        }
        for ent in to_remove {
            duckings.remove(ent);
        }
        for (player, coll_points, entity, grounded) in
            (&players, &mut collisions, &entities, &groundeds).join()
        {
            if player.0 && grounded.0 {
                if input.action_status("vertical").axis < threshold {
                    if !duckings.contains(entity) {
                        let mut new_half = coll_points.half_size;
                        new_half.y /= 2.0;
                        duckings
                            .insert(entity, Ducking::new(new_half, coll_points.half_size))
                            .expect("Failed to insert Ducking component.");
                        coll_points.shrink_height_collision_points(new_half.y);
                    }
                }
            }
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
        Read<'s, InputManager>,
        Read<'s, Time>,
    );

    fn run(&mut self, (mut velocities, grounded, gravities, input, time): Self::SystemData) {
        for (velocity, grounded, gravity) in
            (&mut velocities, (&grounded).maybe(), &gravities).join()
        {
            let grav_dir = gravity.0;

            let mut grav_vel =
                gravitationally_de_adapted_velocity(&velocity.vel, &GravityDirection(grav_dir));

            // Apply friction and slow down
            if let Some(ground) = grounded {
                if ground.0 {
                    let horizontal_movement = input.action_status("horizontal").axis;
                    // Not moving or trying to move in opposite direction
                    if horizontal_movement == 0.0 || horizontal_movement * grav_vel.x < 0.0 {
                        if grav_vel.x.abs() <= 0.05 {
                            grav_vel.x = 0.0;
                        } else {
                            // Slow in opposite direction
                            grav_vel.x -= grav_vel.x * FRICTION * time.time_scale();
                        }
                        velocity.vel = gravitationally_adapted_velocity(
                            &grav_vel,
                            &GravityDirection(grav_dir),
                        );
                    }
                }
            }

            let mut gravity_vec = Vec2::new(0.0, -0.4);
            gravity_vec =
                gravitationally_adapted_velocity(&gravity_vec, &GravityDirection(grav_dir));

            velocity.vel.x += gravity_vec.x * time.time_scale();
            velocity.vel.y += gravity_vec.y * time.time_scale();

            // Limit speed
            velocity.vel.x = f32::min(velocity.vel.x, MAX_RUN_SPEED);
            velocity.vel.x = f32::max(velocity.vel.x, -MAX_RUN_SPEED);

            velocity.vel.y = f32::max(velocity.vel.y, -MAX_FALL_SPEED);
        }
    }
}

#[derive(SystemDesc)]
pub struct ActorCollisionSystem;

impl ActorCollisionSystem {
    fn create_corners_with_coll_points_tl_br(
        pos: &Vec2,
        points: &PlatformCollisionPoints,
    ) -> (Vec2, Vec2) {
        let mut leftmost: f32 = 999999.0;
        let mut rightmost: f32 = -999999.0;
        let mut topmost: f32 = -999999.0;
        let mut bottommost: f32 = 999999.0;
        // Go through every collision point to create cuboid shape
        for collider_offset in &points.collision_points {
            let point_pos = Vec2::new(
                collider_offset.point.x + pos.x,
                collider_offset.point.y + pos.y,
            );

            leftmost = leftmost.min(point_pos.x);
            bottommost = bottommost.min(point_pos.y);
            rightmost = rightmost.max(point_pos.x);
            topmost = topmost.max(point_pos.y);
        }
        //info!("TopLeft: {:?}, BottomRight: {:?}", Vec2::new(leftmost, topmost), Vec2::new(rightmost, bottommost));
        (
            Vec2::new(leftmost, topmost),
            Vec2::new(rightmost, bottommost),
        )
    }

    pub fn cuboid_intersection(
        top_left1: &Vec2,
        bottom_right1: &Vec2,
        top_left2: &Vec2,
        bottom_right2: &Vec2,
    ) -> bool {
        // Two conditions to know if rectangles don't overlap:
        // 1: One of them is above the other
        if top_left1.y <= bottom_right2.y || bottom_right1.y >= top_left2.y {
            return false;
        }

        // 2: One of them is to the left of the other
        if top_left1.x >= bottom_right2.x || bottom_right1.x <= top_left2.x {
            return false;
        }

        true
    }
}

impl<'s> System<'s> for ActorCollisionSystem {
    type SystemData = (
        ReadStorage<'s, Position>,
        ReadStorage<'s, PlatformCollisionPoints>,
        ReadStorage<'s, Team>,
        ReadStorage<'s, Damage>,
        ReadStorage<'s, Projectile>,
        ReadStorage<'s, Reflect>,
        ReadStorage<'s, Block>,
        Entities<'s>,
        Write<'s, EventChannel<CollisionEvent>>,
    );

    fn run(
        &mut self,
        (
            positions,
            coll_points,
            teams,
            damages,
            projectiles,
            reflects,
            blocks,
            entities,
            mut channel,
        ): Self::SystemData,
    ) {
        let mut result = Vec::new();
        for (ent_pos1, coll_point1, entity1) in (&positions, &coll_points, &entities).join() {
            let pos1 = Vec2::new(ent_pos1.0.x, ent_pos1.0.y);
            let (top_left1, bottom_right1) =
                Self::create_corners_with_coll_points_tl_br(&pos1, coll_point1);

            for (ent_pos2, coll_point2, entity2) in (&positions, &coll_points, &entities).join() {
                if entity1 == entity2 {
                    continue;
                }

                let pos2 = Vec2::new(ent_pos2.0.x, ent_pos2.0.y);
                let (top_left2, bottom_right2) =
                    Self::create_corners_with_coll_points_tl_br(&pos2, coll_point2);
                if Self::cuboid_intersection(&top_left1, &bottom_right1, &top_left2, &bottom_right2)
                {
                    let team1 = teams.get(entity1);
                    let team2 = teams.get(entity2);
                    if team1.is_some() && team2.is_some() {
                        let team1 = team1.unwrap();
                        let team2 = team2.unwrap();
                        match (team1, team2) {
                            (Team::GoodGuys, Team::GoodGuys) => {}
                            (Team::BadGuys, Team::BadGuys) => {}
                            (Team::Neutral, _) => {}
                            (_, Team::Neutral) => {}
                            _ => {
                                // It's not necessary to check both permutations, the outer loop does this already
                                if let Some(damage) = damages.get(entity1) {
                                    result.push(CollisionEvent::EnemyCollision(
                                        entity2.id(),
                                        entity1.id(),
                                        damage.0,
                                    ));
                                }
                                if reflects.get(entity1).is_some()
                                    && projectiles.get(entity2).is_some()
                                {
                                    result.push(CollisionEvent::ProjectileReflection(
                                        entity2.id(),
                                        *team1,
                                    ));
                                }
                                if blocks.get(entity1).is_some()
                                    && projectiles.get(entity2).is_some()
                                {
                                    result.push(CollisionEvent::ProjectileBlock(entity2.id()));
                                }
                            }
                        }
                    }
                }
            }
        }
        let block_ids: HashSet<u32> = result
            .iter()
            .map(|e| match e {
                CollisionEvent::ProjectileBlock(id) => *id,
                _ => 999999999,
            })
            .collect();
        let no_collide_when_block: Vec<&CollisionEvent> = result
            .iter()
            .filter(|e| match e {
                CollisionEvent::EnemyCollision(_, projectile, _) => !block_ids.contains(projectile),
                _ => true,
            })
            .collect();
        for event in no_collide_when_block {
            channel.single_write(*event);
        }
    }
}

#[derive(SystemDesc)]
pub struct PlatformCollisionSystem;

impl PlatformCollisionSystem {
    fn raycast(
        point: CollisionPoint,
        vel: &Vec2,
        cuboid_pos: &Vec2,
        cuboid: &PlatformCuboid,
    ) -> Option<(Vec2, CollisionSideOfBlock)> {
        let x_intersects = cuboid.within_range_x(
            &point.point,
            cuboid_pos,
            match point.is_horizontal {
                true => point.half_reach,
                false => 0.,
            },
        );
        let y_intersects = cuboid.within_range_y(
            &point.point,
            cuboid_pos,
            match point.is_horizontal {
                true => 0.,
                false => point.half_reach,
            },
        );

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

        let offset_x = match point.is_horizontal {
            true => point.half_reach,
            false => 0.,
        };
        let offset_y = match point.is_horizontal {
            true => 0.,
            false => point.half_reach,
        };
        let mut hor_dist = hor_side - point.point.x;
        let mut ver_dist = ver_side - point.point.y;
        if hor_side < point.point.x + offset_x && hor_side > point.point.x - offset_x {
            hor_dist = 0.;
        }
        if ver_side < point.point.y + offset_y && ver_side > point.point.y - offset_y {
            ver_dist = 0.;
        }

        // let hor_dist = hor_side - point.x;
        // let ver_dist = ver_side - point.y;

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
            true => Vec2::new(
                hor_side,
                point.point.y + horizontal_perc_to_collision * vel.y,
            ),
            false => Vec2::new(point.point.x + vertical_perc_to_collision * vel.x, ver_side),
        };

        debug!(
            "Percentages = x:{}, y:{}",
            horizontal_perc_to_collision, vertical_perc_to_collision
        );
        debug!("Values = {:?}", result);

        match cuboid.within_range_x(&result, &cuboid_pos, offset_x)
            && cuboid.within_range_y(&result, &cuboid_pos, offset_y)
        {
            true => Some((result, side)),
            false => None,
        }
        // match cuboid.intersect_x(&result, &cuboid_pos) && cuboid.intersect_y(&result, &cuboid_pos) {
        //     true => Some((result, side)),
        //     false => None,
        // }
    }
}

impl<'s> System<'s> for PlatformCollisionSystem {
    type SystemData = (
        WriteStorage<'s, Velocity>,
        WriteStorage<'s, Collidee>,
        ReadStorage<'s, Position>,
        ReadStorage<'s, PlatformCuboid>,
        ReadStorage<'s, PlatformCollisionPoints>,
        Read<'s, RTree<RTreeEntity>>,
        Read<'s, Time>,
    );

    fn run(
        &mut self,
        (mut velocities, mut collidees, positions, cuboids, coll_points, rtree, time): Self::SystemData,
    ) {
        for (velocity, collidee, ent_pos, collision_points) in
            (&mut velocities, &mut collidees, &positions, &coll_points).join()
        {
            // Reset collidees here they can be used for the rest of the frame
            std::mem::swap(&mut collidee.prev_horizontal, &mut collidee.horizontal);
            std::mem::swap(&mut collidee.prev_vertical, &mut collidee.vertical);
            collidee.horizontal = None;
            collidee.vertical = None;
            // We want to loop up to twice here
            // First loop finds the closest collision of all the points
            // Second loop tries to find a collision in the other axis
            //  given the changes in position and velocity
            // These bad boys get modified at the end of the loop
            let mut current_vel = velocity.project_move(time.time_scale());
            let mut current_ent_pos = ent_pos.0.to_vec2().clone();
            loop {
                debug!("Velocity: {:?}", current_vel);
                // We want the shortest distance collision of all points
                let mut prev_distance_to_collision = 9999.0;
                let mut details = None;
                let mut num_coll_points = 0;

                // Go through every collision point
                for col_point in &collision_points.collision_points {
                    let col_point: &CollisionPoint = col_point;
                    let point_pos = Vec2::new(
                        col_point.point.x + current_ent_pos.x,
                        col_point.point.y + current_ent_pos.y,
                    );
                    debug!("Point: {:?}", point_pos);

                    let bottom_left = [
                        current_ent_pos.x + current_vel.x + collision_points.half_size.x * 2.,
                        current_ent_pos.y + current_vel.y + collision_points.half_size.y * 2.,
                    ];
                    let top_right = [
                        current_ent_pos.x + current_vel.x - collision_points.half_size.x * 2.,
                        current_ent_pos.y + current_vel.y - collision_points.half_size.y * 2.,
                    ];
                    for rtree_ent in rtree.locate_in_envelope_intersecting(&AABB::from_corners(
                        bottom_left,
                        top_right,
                    )) {
                        let plat_pos = positions.get(rtree_ent.entity);
                        let cuboid = cuboids.get(rtree_ent.entity);
                        if plat_pos.is_none() || cuboid.is_none() {
                            continue;
                        }
                        let plat_pos = plat_pos.unwrap();
                        let cuboid = cuboid.unwrap();

                        // uncomment the following 2 lines to return to old iterative collisions
                        // }
                        // for (plat_pos, cuboid) in (&positions, &cuboids).join() {
                        // delta so if we go through the corner of a block we still check
                        // Must stay gt or eq to the max speed that will be reached
                        let delta = TILE_WIDTH;
                        let platform_position = plat_pos.0.to_vec2();
                        let point_vel_pos = Vec2::new(
                            col_point.point.x + ent_pos.0.x + current_vel.x,
                            col_point.point.y + ent_pos.0.y + current_vel.y,
                        );

                        // Is the block even close to us
                        if cuboid.within_range_x(&point_vel_pos, &platform_position, delta) {
                            if cuboid.within_range_y(&point_vel_pos, &platform_position, delta) {
                                // point of collision and side
                                let point_of_collision = Self::raycast(
                                    CollisionPoint::new(
                                        point_pos,
                                        col_point.half_reach,
                                        col_point.is_horizontal,
                                    ),
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
                                num_coll_points += 1;

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
                                        &col_point.point,
                                    ),
                                    old_collider_vel: current_vel.clone(),
                                    new_collider_vel: new_velocity,
                                    side: side.clone(),
                                    num_points_of_collision: num_coll_points,
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
                            //println!("BOTH {:?}", collidee.vertical.as_ref().unwrap().old_collider_vel);
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
        WriteStorage<'s, Collidee>,
        WriteStorage<'s, Grounded>,
        ReadStorage<'s, GravityDirection>,
    );

    fn run(&mut self, (mut velocities, mut collidees, mut grounded, gravities): Self::SystemData) {
        for (velocity, collidee, mut grounded, gravity) in (
            &mut velocities,
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
                        if let Some(ground) = &mut grounded {
                            ground.0 = grav_dir == CollisionDirection::FromBottom;
                        };
                    }
                    CollisionSideOfBlock::Top => {
                        if let Some(ground) = &mut grounded {
                            ground.0 = grav_dir == CollisionDirection::FromTop;
                        };
                    }
                    _ => {}
                }
                velocity.vel.y = cdee.correction;
            }

            if let Some(cdee) = &collidee.horizontal {
                match cdee.side {
                    CollisionSideOfBlock::Left => {
                        if let Some(ground) = &mut grounded {
                            ground.0 = ground.0 || grav_dir == CollisionDirection::FromLeft;
                        };
                    }
                    CollisionSideOfBlock::Right => {
                        if let Some(ground) = &mut grounded {
                            ground.0 = ground.0 || grav_dir == CollisionDirection::FromRight;
                        };
                    }
                    _ => {}
                }
                velocity.vel.x = cdee.correction;
            }
        }
    }
}
