mod d3d12;
#[cfg(feature = "windows-ui-composition")]
mod windows_ui_composition;

pub use d3d12::{swap_chain::SkiaD3d12SwapChain, D3d12Backend};

#[cfg(feature = "windows-ui-composition")]
pub use windows_ui_composition::{
    WindowsUiCompositionBackend, WindowsUiCompositionSwapChain, WindowsUiCompositionTarget,
};
