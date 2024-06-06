mod d3d12;
#[cfg(feature = "direct-composition")]
mod dcomp;
#[cfg(feature = "windows-ui-composition")]
mod windows_ui_composition;

pub use d3d12::{
    swap_chain::{HwndSwapChain, SwapChain},
    Backend,
};

#[cfg(feature = "windows-ui-composition")]
pub use windows_ui_composition::{CompositionBackend, CompositionSwapChain, CompositionTarget};

#[cfg(feature = "direct-composition")]
pub use dcomp::{DCompBackend, DCompSwapChain};
