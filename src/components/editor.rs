use crate::utils::Vec2;
use amethyst::{
    assets::{AssetStorage, Handle, Loader},
    core::transform::Transform,
    ecs::prelude::{Component, DenseVecStorage, NullStorage},
    prelude::*,
    renderer::{Camera, ImageFormat, SpriteRender, SpriteSheet, SpriteSheetFormat, Texture},
};

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

// Represents the cursor's position as a dot in the middle of the smallest grid unit it is truly in
pub struct RealCursorPosition(pub Vec2);

impl Component for RealCursorPosition {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default)]
pub struct SizeForEditorGrid(pub Vec2);

impl Component for SizeForEditorGrid {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Clone)]
pub struct CursorWasInThisEntity(pub Option<u32>);

impl Component for CursorWasInThisEntity {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Default, Clone)]
pub struct RealEntityId(pub Option<u32>);

impl Component for RealEntityId {
    type Storage = DenseVecStorage<Self>;
}
