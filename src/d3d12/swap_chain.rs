use skia_safe::{gpu::BackendRenderTarget, Canvas, Surface};
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dxgi::{
        Common::{
            DXGI_ALPHA_MODE, DXGI_ALPHA_MODE_UNSPECIFIED, DXGI_FORMAT_R8G8B8A8_UNORM,
            DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
        },
        IDXGISwapChain3, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        DXGI_USAGE_RENDER_TARGET_OUTPUT,
    },
};

use crate::d3d12::Backend;

pub(crate) const BUFFER_COUNT: u32 = 2;

pub(crate) type SwapChainSurfaceArray = [(Surface, BackendRenderTarget); BUFFER_COUNT as _];

pub struct HwndSwapChain {
    hwnd: HWND,
    swap_chain: SwapChainState,
}
impl HwndSwapChain {
    pub(crate) fn new(hwnd: HWND, swap_chain: SwapChain) -> Self {
        Self {
            hwnd,
            swap_chain: SwapChainState::Active(swap_chain),
        }
    }
    pub fn resize(&mut self, env: &mut Backend, width: u32, height: u32) {
        self.swap_chain.resize(env, width, height);
    }
    pub fn draw(&mut self, env: &mut Backend, f: impl FnMut(&Canvas)) -> windows::core::Result<()> {
        self.recreate_if_needed(env)?;

        unsafe { self.swap_chain.get_active_mut().unwrap_unchecked() }
            .draw(env, f)
            .ok()
    }
    pub fn get_surface(&mut self, env: &mut Backend) -> windows::core::Result<&mut Surface> {
        self.recreate_if_needed(env)?;

        Ok(self.swap_chain.get_active_mut().unwrap().get_surface())
    }
    pub fn present(&mut self, env: &mut Backend) {
        if let Some(swap_chain) = self.swap_chain.get_active_mut() {
            swap_chain.present(env);
        }
    }
    fn recreate_if_needed(&mut self, env: &mut Backend) -> windows::core::Result<()> {
        if let Some((width, height)) = self.swap_chain.needs_resize() {
            env.recreate_context_if_needed()?;

            self.swap_chain =
                SwapChainState::Active(env.create_swap_chain_for_hwnd(self.hwnd, width, height)?);
        }
        Ok(())
    }
}

pub(crate) enum SwapChainState {
    Active(SwapChain),
    Resizing { new_width: u32, new_height: u32 },
}
impl SwapChainState {
    pub(crate) fn get_active(&self) -> Option<&SwapChain> {
        match self {
            Self::Active(swap_chain) => Some(swap_chain),
            _ => None,
        }
    }
    pub(crate) fn get_active_mut(&mut self) -> Option<&mut SwapChain> {
        match self {
            Self::Active(swap_chain) => Some(swap_chain),
            _ => None,
        }
    }
    pub(crate) fn needs_resize(&self) -> Option<(u32, u32)> {
        match self {
            Self::Resizing {
                new_width,
                new_height,
            } => Some((*new_width, *new_height)),
            _ => None,
        }
    }
    pub(crate) fn resize(&mut self, env: &mut Backend, width: u32, height: u32) {
        let needs_resize = self
            .get_active_mut()
            .map(|swap_chain| {
                if swap_chain.resize(env, width, height).is_err() {
                    env.release_context();
                    true
                } else {
                    false
                }
            })
            .unwrap_or(true);

        if needs_resize {
            *self = Self::Resizing {
                new_width: width,
                new_height: height,
            }
        }
    }
}

pub struct SwapChain {
    pub(crate) swap_chain: IDXGISwapChain3,
    surfaces: Option<SwapChainSurfaceArray>,
}

impl SwapChain {
    pub(crate) fn new(swap_chain: IDXGISwapChain3, surfaces: SwapChainSurfaceArray) -> Self {
        Self {
            swap_chain,
            surfaces: Some(surfaces),
        }
    }
    pub fn resize(
        &mut self,
        env: &mut Backend,
        width: u32,
        height: u32,
    ) -> windows::core::Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        env.cleanup();

        self.surfaces = None;

        unsafe {
            self.swap_chain
                .ResizeBuffers(BUFFER_COUNT, width, height, DXGI_FORMAT_UNKNOWN, 0)
        }?;

        self.surfaces
            .replace(env.create_swap_chain_surfaces(&self.swap_chain, width, height));
        Ok(())
    }
    pub fn draw(
        &mut self,
        env: &mut Backend,
        mut f: impl FnMut(&Canvas),
    ) -> windows::core::HRESULT {
        let index = unsafe { self.swap_chain.GetCurrentBackBufferIndex() };
        let surface = &mut self.surfaces.as_mut().unwrap()[index as usize].0;

        let canvas = surface.canvas();

        f(&canvas);

        env.flush_and_submit_surface(surface, None);
        unsafe { self.swap_chain.Present(1, 0) }
    }
    pub fn present(&mut self, env: &mut Backend) {
        let surface = self.get_surface();
        env.flush_and_submit_surface(surface, None);
        unsafe { self.swap_chain.Present(1, 0) }.ok().unwrap()
    }
    pub fn get_surface(&mut self) -> &mut Surface {
        let index = unsafe { self.swap_chain.GetCurrentBackBufferIndex() };
        &mut self.surfaces.as_mut().unwrap()[index as usize].0
    }
}

pub(crate) fn swap_chain_desc_hwnd(width: u32, height: u32) -> DXGI_SWAP_CHAIN_DESC1 {
    swap_chain_desc(width, height, DXGI_ALPHA_MODE_UNSPECIFIED)
}

pub(crate) fn swap_chain_desc_composition(width: u32, height: u32) -> DXGI_SWAP_CHAIN_DESC1 {
    swap_chain_desc(width, height, DXGI_ALPHA_MODE_UNSPECIFIED)
}

fn swap_chain_desc(width: u32, height: u32, alpha_mode: DXGI_ALPHA_MODE) -> DXGI_SWAP_CHAIN_DESC1 {
    DXGI_SWAP_CHAIN_DESC1 {
        Width: width,
        Height: height,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: BUFFER_COUNT,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        AlphaMode: alpha_mode,
        ..Default::default()
    }
}
