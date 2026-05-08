use std::error::Error;
use std::fmt;

use crate::Pixels;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerShellOptions {
    pub namespace: String,
    pub layer: Layer,
    pub anchor: Anchor,
    pub margin: Option<(Pixels, Pixels, Pixels, Pixels)>,
    pub keyboard_interactivity: KeyboardInteractivity,
    pub exclusive_zone: Option<Pixels>,
    pub exclusive_edge: Option<Anchor>,
}

impl Default for LayerShellOptions {
    fn default() -> Self {
        Self {
            namespace: String::new(),
            layer: Layer::Top,
            anchor: Anchor::empty(),
            margin: None,
            keyboard_interactivity: KeyboardInteractivity::None,
            exclusive_zone: None,
            exclusive_edge: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layer {
    Background,
    Bottom,
    #[default]
    Top,
    Overlay,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Anchor: u32 {
        const TOP = 1;
        const BOTTOM = 2;
        const LEFT = 4;
        const RIGHT = 8;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardInteractivity {
    #[default]
    None,
    Exclusive,
    OnDemand,
}

#[derive(Debug)]
pub struct LayerShellNotSupportedError;

impl fmt::Display for LayerShellNotSupportedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Wayland layer shell is not supported by this compositor")
    }
}

impl Error for LayerShellNotSupportedError {}
