use derivative::Derivative;
use ultraviolet::Vec2;
use amethyst_core::ecs::{DenseVecStorage, Component, Entity};
use ultraviolet::wide::{Mul, Add, AddAssign};
use rstar::{RTreeObject, AABB};

/// When inserted as a `Resource` into `World`,
/// affects the base strength of gravity on `RigidBody` components.
pub struct Gravity {
    pub strength: f32,
}

#[derive(Derivative, Component)]
#[derivative(Default)]
pub struct RigidBody {
    pub velocity: Vec2,
    #[derivative(Default(value = "Vec2::new(0.0, 1.0)"))]
    pub gravity: Vec2,
}

impl RigidBody {
    pub fn project_move(&self, time_scale: f32) -> Vec2 {
        self.velocity.mul(time_scale)
    }
}

#[derive(Component, Clone)]
pub struct Collider {
    pub position: Vec2,
    pub lower: Vec2,
    pub upper: Vec2,
    pub is_trigger: bool,
    pub(crate) projected_movement: Vec2,
}
impl Collider {
    pub fn intersects(&self, other: &Self) -> bool {
        (self.lower.x <= other.upper.x && self.upper.x >= other.lower.x)
            && (self.lower.y <= other.upper.y && self.upper.y >= other.lower.y)
    }

    fn project(&self) -> (Vec2, Vec2) {
        let projected = self.position.add(self.projected_movement);
        (self.lower.add(projected), self.upper.add(projected))
    }
}

impl RTreeObject for Collider {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let corners = self.project();
        AABB::from_corners(corners.0, corners.1)
    }
}

#[derive(Component)]
pub struct ChildTo {
    pub parent: Entity,
    pub offset: Vec2,
}

