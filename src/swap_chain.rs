use skia_safe::{gpu::BackendRenderTarget, Canvas, Surface};
use windows::Win32::Graphics::Dxgi::{Common::DXGI_FORMAT_UNKNOWN, IDXGISwapChain3};

use crate::D3d12Backend;

pub(crate) const BUFFER_COUNT: u32 = 2;

pub(crate) type SkiaD3d12SwapChainSurfaceArray =
    [(Surface, BackendRenderTarget); BUFFER_COUNT as _];

pub struct SkiaD3d12SwapChain {
    pub(crate) swap_chain: IDXGISwapChain3,
    surfaces: Option<SkiaD3d12SwapChainSurfaceArray>,
}

impl SkiaD3d12SwapChain {
    pub(crate) fn new(
        swap_chain: IDXGISwapChain3,
        surfaces: SkiaD3d12SwapChainSurfaceArray,
    ) -> Self {
        Self {
            swap_chain,
            surfaces: Some(surfaces),
        }
    }
    pub fn resize(
        &mut self,
        env: &mut D3d12Backend,
        width: u32,
        height: u32,
    ) -> windows::core::Result<()> {
        env.cleanup();

        self.surfaces = None;

        unsafe {
            self.swap_chain
                .ResizeBuffers(BUFFER_COUNT, width, height, DXGI_FORMAT_UNKNOWN, 0)
        }
        .unwrap();

        self.surfaces
            .replace(env.create_swap_chain_surfaces(&self.swap_chain, width, height));
        Ok(())
    }
    pub fn draw(
        &mut self,
        env: &mut D3d12Backend,
        mut f: impl FnMut(&Canvas),
    ) -> windows::core::HRESULT {
        let index = unsafe { self.swap_chain.GetCurrentBackBufferIndex() };
        let surface = &mut self.surfaces.as_mut().unwrap()[index as usize].0;

        let canvas = surface.canvas();

        f(&canvas);

        env.direct_context.flush_and_submit_surface(surface, None);
        unsafe { self.swap_chain.Present(1, 0) }
    }
    pub fn present(&mut self, env: &mut D3d12Backend) {
        let surface = self.get_surface();
        env.direct_context.flush_and_submit_surface(surface, None);
        unsafe { self.swap_chain.Present(1, 0) }.ok().unwrap()
    }
    pub fn get_surface(&mut self) -> &mut Surface {
        let index = unsafe { self.swap_chain.GetCurrentBackBufferIndex() };
        &mut self.surfaces.as_mut().unwrap()[index as usize].0
    }
}
