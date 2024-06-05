mod d3d12;
mod swap_chain;
#[cfg(feature = "windows-ui-composition")]
mod windows_ui_composition;

pub use {d3d12::D3d12Backend, swap_chain::SkiaD3d12SwapChain};

#[cfg(feature = "windows-ui-composition")]
pub use windows_ui_composition::{WindowsUiCompositionBackend, WindowsUiCompositionTarget};
