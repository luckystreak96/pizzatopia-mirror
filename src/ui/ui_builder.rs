use crate::states::pizzatopia::DEPTH_UI;
use crate::ui::{with_transparent, COLOR_BLACK};
use amethyst::assets::Handle;
use amethyst::core::ecs::{Entity, EntityBuilder};
use amethyst::prelude::*;
use amethyst::ui::{Anchor, FontAsset, UiImage, UiText, UiTransform};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use ultraviolet::Vec2;

/// Make sure you add a `Position` component to this
/// as the position is at -1000, -1000 by default
pub fn initialize_ui_label(world: &mut World, text: String, font_size: f32) -> EntityBuilder {
    let font = (*world.read_resource::<Handle<FontAsset>>()).clone();

    let ui_text = create_ui_text(text.clone(), font_size, font);
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let id = id.to_string();
    let transform = create_ui_transform(
        id,
        -1000.,
        -1000.,
        font_size / 2. * text.len() as f32,
        font_size,
    );
    create_ui_entity(world, transform, ui_text)
}

fn create_ui_text(text: String, font_size: f32, font: Handle<FontAsset>) -> UiText {
    let mut text = UiText::new(
        font,
        format!("{}", text).to_string(),
        [1., 1., 1., 1.],
        font_size,
    );
    text.align = Anchor::Middle;
    text
}

fn create_ui_transform(id: String, x: f32, y: f32, width: f32, height: f32) -> UiTransform {
    let transform = UiTransform::new(
        id,
        Anchor::Middle,
        Anchor::Middle,
        x,
        y,
        DEPTH_UI,
        width,
        height,
    );
    transform
}

fn create_ui_entity(world: &mut World, transform: UiTransform, text: UiText) -> EntityBuilder {
    world.create_entity().with(transform).with(text)
}
