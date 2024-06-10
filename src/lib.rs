mod d3d12;
#[cfg(feature = "direct-composition")]
mod dcomp;
#[cfg(feature = "windows-ui-composition")]
mod wincomp;

pub use d3d12::{
    swap_chain::{HwndSwapChain, SwapChain},
    Backend,
};

#[cfg(feature = "windows-ui-composition")]
pub use wincomp::{WinCompBackend, WinCompSwapChain, WinCompTarget};

#[cfg(feature = "direct-composition")]
pub use dcomp::{DCompBackend, DCompSwapChain};
