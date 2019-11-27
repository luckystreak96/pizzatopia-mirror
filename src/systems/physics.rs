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
    fn raycast(point: &Vec2, vel: &Vec2, pos: &Vec2, cuboid: &PlatformCuboid) -> Option<Vec2> {
        let x_intersects = Self::intersect_x(point, pos, cuboid);
        let y_intersects = Self::intersect_y(point, pos, cuboid);

        // The point must be outside the tile
        if x_intersects && y_intersects {
            error!("Already inside block!");
            return None;
        }

        // Find out which sides of the block we want to use
        let mut hor_side = pos.x;
        let mut ver_side = pos.y;
        match vel.x > 0.0 {
            true => hor_side -= cuboid.half_width,
            false => hor_side += cuboid.half_width,
        };
        match vel.y > 0.0 {
            true => ver_side -= cuboid.half_height,
            false => ver_side += cuboid.half_height,
        };

        let hor_dist = hor_side - point.x;
        let ver_dist = ver_side - point.y;

        let magic_number = -9999.0;

        let mut vertical_perc_to_collision = match vel.y != 0.0 {
            true => ver_dist / vel.y,
            false => match y_intersects {
                true => magic_number,
                false => {
                    error!(
                        "Division by 0 and no y-collision! Point: {:?}, Block: {:?}",
                        point, pos
                    );
                    return None;
                }
            },
        };
        info!("Point: {:?}, Block: {:?}", point, pos);
        warn!(
            "Percentage calc vertical: distance({}) / vel({})",
            ver_dist, vel.y
        );

        let mut horizontal_perc_to_collision = match vel.x != 0.0 {
            true => hor_dist / vel.x,
            false => match x_intersects {
                true => magic_number,
                false => {
                    error!(
                        "Division by 0 and no x-collision! Point: {:?}, Block: {:?}",
                        point, pos
                    );
                    return None;
                }
            },
        };

        warn!(
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

        // There's no collision at all, they both go to infinity
        if horizontal_perc_to_collision == magic_number
            && vertical_perc_to_collision == magic_number
        {
            return None;
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

        if horizontal_perc_to_collision >= 0.0 && horizontal_perc_to_collision <= 1.0
            || vertical_perc_to_collision <= 1.0 && vertical_perc_to_collision >= 0.0
        {
        } else {
            return None;
        }

        info!(
            "Percentages = x:{}, y:{}",
            horizontal_perc_to_collision, vertical_perc_to_collision
        );
        info!(
            "Values = {:?}",
            match horizontal_perc_to_collision > vertical_perc_to_collision {
                true => Vec2::new(hor_side, point.y + horizontal_perc_to_collision * vel.y),
                false => Vec2::new(point.x + vertical_perc_to_collision * vel.x, ver_side),
            }
        );

        let result = match horizontal_perc_to_collision > vertical_perc_to_collision {
            true => Vec2::new(hor_side, point.y + horizontal_perc_to_collision * vel.y),
            false => Vec2::new(point.x + vertical_perc_to_collision * vel.x, ver_side),
        };

        match Self::intersect_x(&result, &pos, cuboid) && Self::intersect_y(&result, &pos, cuboid) {
            true => Some(result),
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

    fn can_collide_with_vel(
        horizontal: bool,
        point: &Vec2,
        vel: &Vec2,
        pos: &Vec2,
        cuboid: &PlatformCuboid,
    ) -> bool {
        let new_pos = Vec2::new(point.x + vel.x, point.y + vel.y);
        match horizontal {
            true => Self::intersect_y(&new_pos, pos, cuboid),
            false => Self::intersect_x(&new_pos, pos, cuboid),
        }
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

    // ---------------------------------------------------
    // TODO: Make this all work with collidee!!!
    // ---------------------------------------------------
    fn run(
        &mut self,
        (mut velocities, mut collidees, positions, cuboids, coll_points): Self::SystemData,
    ) {
        for (velocity, collidee, ent_pos, coll_point) in
            (&mut velocities, &mut collidees, &positions, &coll_points).join()
        {
            let mut prev_distance_to_collision = Vec2::new(9999.0, 9999.0);
            for point in &coll_point.0 {
                info!(
                    "Point: {:?}",
                    Vec2::new(ent_pos.0.x + point.x, ent_pos.0.y + point.y)
                );
                let vel = velocity.0.clone();
                let mut side_confirmed = None;

                let point_pos = Vec2::new(point.x + ent_pos.0.x, point.y + ent_pos.0.y);
                for (plat_pos, cuboid) in (&positions, &cuboids).join() {
                    // delta so if we go through the corner of a block we still check
                    let delta = 5.0;
                    let point_vel_pos =
                        Vec2::new(point.x + ent_pos.0.x + vel.x, point.y + ent_pos.0.y + vel.y);
                    if Self::within_range_x(&point_vel_pos, &plat_pos.0, cuboid, delta) {
                        if Self::within_range_y(&point_vel_pos, &plat_pos.0, cuboid, delta) {
                            let name = String::from("Some block");
                            let position = Vec2::new(plat_pos.0.x, plat_pos.0.y);
                            let half_size = Vec2::new(cuboid.half_width, cuboid.half_height);

                            // point of collision will allow us to identify the side
                            let point_of_collision =
                                Self::raycast(&point_pos, &vel, &position, cuboid);

                            // find the distance to the point to see if another collision was closer
                            let cur_distance_to_collision = match &point_of_collision {
                                Some(c) => (c.x.powi(2) + c.y.powi(2)).sqrt(),
                                None => continue,
                            };

                            let side;

                            let coll_pnt = &point_of_collision.unwrap();
                            info!("ColPoint: {:?}, Position: {:?}", coll_pnt, position);
                            // The collision is vertical
                            if coll_pnt.y == position.y + half_size.y
                                || coll_pnt.y == position.y - half_size.y
                            {
                                side = match vel.y > 0.0 {
                                    true => CollisionSideOfBlock::Bottom,
                                    false => CollisionSideOfBlock::Top,
                                };
                            }
                            // The collision is horizontal
                            // If both vert and horizontal are equal - vertical is chosen
                            else {
                                side = match vel.x > 0.0 {
                                    true => CollisionSideOfBlock::Left,
                                    false => CollisionSideOfBlock::Right,
                                };
                            }

                            match side.is_horizontal() {
                                true => {
                                    match cur_distance_to_collision > prev_distance_to_collision.x {
                                        true => continue,
                                        false => {
                                            // Check if another collision exists
                                            if let Some(ver) = &collidee.vertical {
                                                // If that collision happens before you, ensure that you adjust vel and re-check
                                                if ver.distance < cur_distance_to_collision {
                                                    let new_vel = Vec2::new(vel.x, 0.0);
                                                    // Discard this collision if collision no longer applies
                                                    if !Self::can_collide_with_vel(
                                                        true, &point_pos, &new_vel, &position,
                                                        cuboid,
                                                    ) {
                                                        continue;
                                                    }
                                                }
                                                // TODO: If that collision happens after, do the reverse thing
                                            }
                                            prev_distance_to_collision.x = cur_distance_to_collision
                                        }
                                    }
                                }
                                false => {
                                    match cur_distance_to_collision > prev_distance_to_collision.y {
                                        true => continue,
                                        false => {
                                            // Check if another collision exists
                                            if let Some(hor) = &collidee.horizontal {
                                                // If that collision happens before you, ensure that you adjust vel and re-check
                                                if hor.distance < cur_distance_to_collision {
                                                    let new_vel = Vec2::new(0.0, vel.y);
                                                    // Discard this collision if collision no longer applies
                                                    if !Self::can_collide_with_vel(
                                                        false, &point_pos, &new_vel, &position,
                                                        cuboid,
                                                    ) {
                                                        continue;
                                                    }
                                                }
                                                // TODO: If that collision happens after, do the reverse thing
                                            }
                                            prev_distance_to_collision.y = cur_distance_to_collision
                                        }
                                    }
                                }
                            }

                            info!("Side: {:?}", side.clone());

                            // Calculate the vertical/horizontal correction to be applied
                            let mut correction = 0.01;
                            match side.clone() {
                                CollisionSideOfBlock::Top => {
                                    correction = coll_pnt.y + correction - point_pos.y
                                }
                                CollisionSideOfBlock::Bottom => {
                                    correction = coll_pnt.y - correction - point_pos.y
                                }
                                CollisionSideOfBlock::Left => {
                                    correction = coll_pnt.x - correction - point_pos.x
                                }
                                CollisionSideOfBlock::Right => {
                                    correction = coll_pnt.x + correction - point_pos.x
                                }
                            };
                            info!("Correction: {}", correction);

                            side_confirmed = Some(side.clone());

                            let details = Some(CollideeDetails {
                                name,
                                position,
                                half_size,
                                correction,
                                distance: cur_distance_to_collision,
                                side: side.clone(),
                            });
                            match side.is_horizontal() {
                                true => collidee.horizontal = details,
                                false => collidee.vertical = details,
                            }
                        }
                    }
                }

                let mut will_apply = true;
                if let Some(hor) = &collidee.horizontal {
                    if side.is_horizontal() {
                        // Check if hor has smaller correction
                        // If so, skip this guy
                        if hor.correction < det.correction {
                            will_apply = false;
                        }
                    }
                } else if let Some(ver) = &collidee.vertical {
                    if side.is_vertical() {
                        // Check if ver has smaller correction
                        // If so, skip this guy
                        if ver.correction < det.correction {
                            will_apply = false;
                        }
                    }
                }

                // If there's 2 collisions with the same block, remove the horizontal collision
                if let Some(hor) = &collidee.horizontal {
                    // TODO: Same block recognition uses position for now
                    if det.position == hor.position {
                        if det.side.is_vertical() {
                            collidee.horizontal = None;
                        }
                    }
                }
                if let Some(vert) = &collidee.vertical {
                    // TODO: Same block recognition uses position for now
                    if det.position == vert.position {
                        if det.side.is_horizontal() {
                            will_apply = false;
                        }
                    }
                }

                if will_apply {
                    match side.is_horizontal() {
                        true => collidee.horizontal = Some(det),
                        false => collidee.vertical = Some(det),
                    };
                    info!("Collided on side: {:?}", side);
                } else {
                    warn!("Will not apply collision!");
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
