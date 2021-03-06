//! This module is included while awaiting an upstream merge in stretch proper.
//! You should not rely on it, and consider it an implementation detail.

use crate::stretch::algo::ComputeResult;
use crate::stretch::geometry::{Point, Size};
use crate::stretch::number::Number;

#[derive(Copy, Debug, Clone)]
pub struct Layout {
    pub(crate) order: u32,
    pub size: Size<f32>,
    pub location: Point<f32>,
}

impl Layout {
    pub(crate) fn new() -> Self {
        Layout { order: 0, size: Size { width: 0.0, height: 0.0 }, location: Point { x: 0.0, y: 0.0 } }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Cache {
    pub(crate) node_size: Size<Number>,
    pub(crate) parent_size: Size<Number>,
    pub(crate) perform_layout: bool,

    pub(crate) result: ComputeResult,
}
