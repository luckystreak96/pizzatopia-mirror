use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn subtract(first: &Vec2, subtract_by: &Vec2) -> Vec2 {
        Vec2 {
            x: first.x - subtract_by.x,
            y: first.y - subtract_by.y,
        }
    }
}

impl Eq for Vec2 {}

impl Ord for Vec2 {
    fn cmp(&self, other: &Vec2) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
}
