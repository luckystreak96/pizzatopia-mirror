pub use pizzatopia_derive::{enum_cycle, EnumCycle};
pub use strum::IntoEnumIterator;
pub use strum_macros::{EnumCount, EnumIter};

pub trait EnumCycle {
    fn next(&self) -> Self;
    fn prev(&self) -> Self;
}
