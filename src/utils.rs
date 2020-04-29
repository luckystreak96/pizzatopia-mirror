use derivative::Derivative;
use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::io;

#[derive(Clone, Copy, Default, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
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
