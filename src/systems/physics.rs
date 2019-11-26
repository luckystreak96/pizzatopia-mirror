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
use log::{error, info, warn};

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
            for (point, groundable) in &coll_point.0 {
                let mut vel = velocity.0.clone();
                // If we collide in 1 direction, check the other
                let mut collision_found = true;
                //warn!("New coll_point!");
                let mut counter = 0;
                while collision_found {
                    counter += 1;
                    if counter > 3 {
                        break;
                    }
                    collision_found = false;
                    // If we snag a corner, check if another block would make collision clear
                    let mut indecision_delay = 0;
                    let mut details: Option<CollideeDetails> = None;
                    let mut side_confirmed = None;

                    let point_pos =
                        Vec2::new(point.x + ent_pos.0.x + vel.x, point.y + ent_pos.0.y + vel.y);

                    for (plat_pos, cuboid) in (&positions, &cuboids).join() {
                        if indecision_delay == 1 {
                            indecision_delay = -1;
                        }
                        let delta = 5.0;
                        if Self::within_range_x(&point_pos, &plat_pos.0, cuboid, delta) {
                            if Self::within_range_y(&point_pos, &plat_pos.0, cuboid, delta) {
                                let name = String::from("Some block");
                                let position = Vec2::new(plat_pos.0.x, plat_pos.0.y);
                                let half_size = Vec2::new(cuboid.half_width, cuboid.half_height);

                                let mut side = None;

                                let iterations = 8;
                                // -1 here so we range from 0 to full
                                let tmp_vel = Vec2::new(
                                    vel.x / (iterations - 1) as f32,
                                    vel.y / (iterations - 1) as f32,
                                );

                                // Split the velocity by x - calculate x dummy frames till 1 collision is possible but not the other
                                let mut has_collided = false;
                                for i in 0..iterations {
                                    let vel_x_frame = vel.x - i as f32 * tmp_vel.x;
                                    let vel_y_frame = vel.y - i as f32 * tmp_vel.y;
                                    let tmp_point = Vec2::new(
                                        point.x + ent_pos.0.x + vel_x_frame,
                                        point.y + ent_pos.0.y + vel_y_frame,
                                    );

                                    let mut horizontal_align = false;
                                    let mut vertical_align = false;

                                    // Is there only 1 possible collision?
                                    // Vertical intersect
                                    if Self::intersect_x(&tmp_point, &plat_pos.0, cuboid) {
                                        vertical_align = true;
                                        //info!("vert=true : {:?} {:?}", tmp_point, plat_pos);
                                    }
                                    // Horizontal intersect
                                    if Self::intersect_y(&tmp_point, &plat_pos.0, cuboid) {
                                        horizontal_align = true;
                                        //info!("hor=true");
                                    }

                                    if !has_collided && horizontal_align && vertical_align {
                                        has_collided = true;
                                        //info!("Collided!");
                                        //info!("Point: {:?} Pos: {:?}", tmp_point, plat_pos.0);
                                    }

                                    if horizontal_align && !vertical_align {
                                        side = match vel.x > 0.0 {
                                            true => Some(CollisionSideOfBlock::Left),
                                            false => Some(CollisionSideOfBlock::Right),
                                        };
                                    //info!("Point: {:?} Pos: {:?}", tmp_point, plat_pos.0);
                                    /*warn!(
                                        "Iteration: {}, Tmp_point: {:?}, vel: {:?}",
                                        i, tmp_point, vel
                                    );*/
                                    } else if vertical_align && !horizontal_align {
                                        side = match vel.y > 0.0 {
                                            true => Some(CollisionSideOfBlock::Bottom),
                                            false => Some(CollisionSideOfBlock::Top),
                                        };
                                        //info!("Point: {:?} Pos: {:?}", tmp_point, plat_pos.0);
                                        /*warn!(
                                            "Iteration: {}, Tmp_point: {:?}, vel: {:?}",
                                            i, tmp_point, vel
                                        );*/
                                    }
                                }
                                //info!("Completed tiles with side = {:?}", side);

                                if !has_collided {
                                    side = None;
                                }

                                if side.is_none() {
                                    match has_collided {
                                        true => {
                                            if indecision_delay == 0 && *groundable {
                                                indecision_delay = 1;
                                                //warn!("Indecision delay used on collision");
                                                vel.y = 0.0;
                                                match ent_pos.0.y > plat_pos.0.y {
                                                    true => side = Some(CollisionSideOfBlock::Top),
                                                    false => {
                                                        side = Some(CollisionSideOfBlock::Bottom)
                                                    }
                                                };
                                            } else {
                                                if vel.x == 0.0 {
                                                    break;
                                                }
                                                match vel.x > 0.0 {
                                                    true => side = Some(CollisionSideOfBlock::Left),
                                                    false => {
                                                        side = Some(CollisionSideOfBlock::Right)
                                                    }
                                                };
                                                vel.x = 0.0;
                                            }
                                        }
                                        false => {
                                            //error!("Side was None!!!");
                                            continue;
                                        }
                                    }
                                }

                                //info!("Side: {:?}", side.clone().unwrap());

                                // Calculate the vertical/horizontal correction to be applied
                                let mut correction = 0.01;
                                match side.clone().unwrap() {
                                    CollisionSideOfBlock::Top => {
                                        correction = (position.y + half_size.y + correction)
                                            - (point.y + ent_pos.0.y)
                                    }
                                    CollisionSideOfBlock::Bottom => {
                                        correction = (position.y - half_size.y - correction)
                                            - (point.y + ent_pos.0.y)
                                    }
                                    CollisionSideOfBlock::Left => {
                                        correction = (position.x - half_size.x - correction)
                                            - (point.x + ent_pos.0.x)
                                    }
                                    CollisionSideOfBlock::Right => {
                                        correction = (position.x + half_size.x + correction)
                                            - (point.x + ent_pos.0.x)
                                    }
                                };
                                //info!("Correction: {}", correction);

                                // Skip this bad boy if he's further away
                                if let Some(det) = &details {
                                    let mut corr_new = correction;
                                    let mut corr_old = det.correction;
                                    if indecision_delay == 1 {
                                        corr_new += 100.0;
                                    }
                                    if indecision_delay == -1 {
                                        corr_old += 100.0;
                                    }
                                    if corr_old.abs() < corr_new.abs() {
                                        //info!("Skipping because one was closer");
                                        continue;
                                    } else {
                                        indecision_delay = -2;
                                    }
                                }

                                side_confirmed = side.clone();

                                details = Some(CollideeDetails {
                                    name,
                                    position,
                                    half_size,
                                    correction,
                                    indecision: indecision_delay == 1,
                                    side: side.clone().unwrap(),
                                });
                            }
                        }
                    }

                    if let Some(side) = side_confirmed {
                        if let Some(det) = details {
                            match side {
                                CollisionSideOfBlock::Right => {
                                    collidee.horizontal = Some(det);
                                    vel.x = 0.0;
                                    //info!("Collided on side: {:?}", side);
                                }
                                CollisionSideOfBlock::Left => {
                                    collidee.horizontal = Some(det);
                                    vel.x = 0.0;
                                    //info!("Collided on side: {:?}", side);
                                }
                                CollisionSideOfBlock::Top => {
                                    collidee.vertical = Some(det);
                                    vel.y = 0.0;
                                    //info!("Collided on side: {:?}", side);
                                }
                                CollisionSideOfBlock::Bottom => {
                                    collidee.vertical = Some(det);
                                    vel.y = 0.0;
                                    //info!("Collided on side: {:?}", side);
                                }
                            };
                            if vel.x == 0.0 && vel.y == 0.0 {
                                collision_found = false;
                            //info!("Collisions done, exiting loop for this point");
                            } else {
                                collision_found = true;
                                //info!("Collisions not done");
                            }
                        }
                    }
                    if collision_found {
                        //info!("Num collisions is greater than 1!");
                    } else {
                        //info!("Num collisions is <= 1");
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
