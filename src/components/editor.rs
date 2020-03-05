use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};
use crate::utils::Vec2;

#[derive(Default)]
pub struct EditorEntity;
impl Component for EditorEntity {
    type Storage = NullStorage<Self>;
}

#[derive(Default)]
pub struct EditorCursor;
impl Component for EditorCursor {
    type Storage = NullStorage<Self>;
}

pub struct RealCursorPosition(pub Vec2);

impl Component for RealCursorPosition {
    type Storage = DenseVecStorage<Self>;
}
