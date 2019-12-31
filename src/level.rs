use crate::utils::Vec2;
use amethyst::{
    assets::{
        Asset, AssetStorage, Format, Handle, Loader, ProcessingState, Processor, ProgressCounter,
        Source,
    },
    ecs::VecStorage,
    error::{format_err, Error, ResultExt},
    prelude::*,
    utils::application_root_dir,
};
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub tiles: Vec<Vec2>,
}

impl Asset for Level {
    const NAME: &'static str = "pizzatopia::level::Level";
    // use `Self` if the type is directly serialized.
    type Data = Self;
    type HandleStorage = VecStorage<Handle<Level>>;
}

impl From<Level> for Result<Level, Error> {
    fn from(level: Level) -> Result<Level, Error> {
        Ok(level)
    }
}
