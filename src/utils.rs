use derivative::Derivative;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, io};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn is_zero(&self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }

    pub fn magnitude(&self) -> f32 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }

    pub fn abs(&self) -> Vec2 {
        Vec2 {
            x: self.x.abs(),
            y: self.y.abs(),
        }
    }

    pub fn mul(&self, other: &Vec2) -> Vec2 {
        Vec2 {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }

    pub fn mul_f32(&self, val: f32) -> Vec2 {
        Vec2 {
            x: self.x * val,
            y: self.y * val,
        }
    }

    pub fn normalize(&self) -> Vec2 {
        let magnitude = self.magnitude();
        Vec2 {
            x: self.x / magnitude,
            y: self.y / magnitude,
        }
    }

    pub fn subtract(first: &Vec2, subtract_by: &Vec2) -> Vec2 {
        Vec2 {
            x: first.x - subtract_by.x,
            y: first.y - subtract_by.y,
        }
    }

    pub fn add(&self, second: &Vec2) -> Vec2 {
        Vec2 {
            x: self.x + second.x,
            y: self.y + second.y,
        }
    }

    pub fn sub(&self, second: &Vec2) -> Vec2 {
        Vec2 {
            x: self.x - second.x,
            y: self.y - second.y,
        }
    }

    pub fn to_vec3(&self) -> Vec3 {
        Vec3 {
            x: self.x,
            y: self.y,
            z: 0.0,
        }
    }
}

impl Eq for Vec2 {}

impl Ord for Vec2 {
    fn cmp(&self, other: &Vec2) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize, Derivative)]
#[derivative(Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }

    pub fn to_vec2(&self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }
}

impl Eq for Vec3 {}

impl Ord for Vec3 {
    fn cmp(&self, other: &Vec3) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub fn read_line_from_console() -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(n) => {
            println!("{} bytes read", n);
            println!("{}", input);
        }
        Err(error) => println!("error: {}", error),
    };
    if !input.is_empty() {
        println!("The text was {}", input);
    }
    input
}
