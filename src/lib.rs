mod d3d12;
#[cfg(feature = "windows-ui-composition")]
mod windows_ui_composition;

pub use d3d12::{swap_chain::SwapChain, Backend};

#[cfg(feature = "windows-ui-composition")]
pub use windows_ui_composition::{CompositionBackend, CompositionSwapChain, CompositionTarget};
