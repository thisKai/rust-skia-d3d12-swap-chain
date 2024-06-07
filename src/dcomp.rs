use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use skia_safe::{Canvas, Surface};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct2D::{D2D1CreateDevice, ID2D1Device},
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                D3D11_SDK_VERSION,
            },
            DirectComposition::{
                DCompositionCreateDevice2, IDCompositionDesktopDevice, IDCompositionTarget,
            },
            Dxgi::{IDXGIDevice3, IDXGISwapChain3},
        },
    },
};

use crate::d3d12::{
    swap_chain::{SwapChain, SwapChainState},
    Backend,
};

pub struct DCompBackend {
    d3d11_device: ID3D11Device,
    pub dcomp_desktop_device: IDCompositionDesktopDevice,
    d3d12: Backend,
}
impl DCompBackend {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let d3d11_device = Self::create_device_3d()?;
            let d2d_device = Self::create_device_2d(&d3d11_device)?;
            let dcomp_desktop_device: IDCompositionDesktopDevice =
                DCompositionCreateDevice2(&d2d_device)?;

            Ok(Self {
                d3d11_device,
                dcomp_desktop_device,
                d3d12: Backend::new()?,
            })
        }
    }
    pub fn create_swap_chain(
        &mut self,
        width: u32,
        height: u32,
    ) -> windows::core::Result<DCompSwapChain> {
        Ok(DCompSwapChain::new(
            self.d3d12
                .create_swap_chain_for_composition(width, height)?,
        ))
    }
    pub fn create_target_for_window<W: HasRawWindowHandle>(
        &mut self,
        window: &W,
    ) -> windows::core::Result<IDCompositionTarget> {
        self.create_target_for_raw_window_handle(window.raw_window_handle())
    }
    pub fn create_target_for_raw_window_handle(
        &mut self,
        window_handle: RawWindowHandle,
    ) -> windows::core::Result<IDCompositionTarget> {
        let hwnd = match window_handle {
            RawWindowHandle::Win32(window_handle) => HWND(window_handle.hwnd as _),
            _ => panic!("not win32"),
        };
        self.create_target_for_hwnd(hwnd)
    }
    pub fn create_target_for_hwnd(
        &mut self,
        hwnd: HWND,
    ) -> windows::core::Result<IDCompositionTarget> {
        unsafe { self.dcomp_desktop_device.CreateTargetForHwnd(hwnd, true) }
    }
    pub fn draw_handler(&self) {}
    fn create_device_3d() -> windows::core::Result<ID3D11Device> {
        let mut device = None;

        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
            .map(|()| device.unwrap())
        }
    }

    fn create_device_2d(device_3d: &ID3D11Device) -> windows::core::Result<ID2D1Device> {
        let dxgi: IDXGIDevice3 = device_3d.cast()?;
        unsafe { D2D1CreateDevice(&dxgi, None) }
    }
}

pub struct DCompSwapChain(SwapChainState);
impl DCompSwapChain {
    fn new(swap_chain: SwapChain) -> Self {
        Self(SwapChainState::Active(swap_chain))
    }
    pub fn resize(&mut self, env: &mut DCompBackend, width: u32, height: u32) {
        self.0.resize(&mut env.d3d12, width, height);
    }
    pub fn new_inner_swap_chain(
        &mut self,
        env: &mut DCompBackend,
    ) -> windows::core::Result<Option<&IDXGISwapChain3>> {
        if let Some((width, height)) = self.0.needs_resize() {
            env.d3d12.recreate_context_if_needed()?;

            let swap_chain = env.d3d12.create_swap_chain_for_composition(width, height)?;

            self.0 = SwapChainState::Active(swap_chain);

            Ok(self.0.get_active().map(|swap_chain| &swap_chain.swap_chain))
        } else {
            Ok(None)
        }
    }
    pub fn unwrap_inner_swap_chain(&self) -> &IDXGISwapChain3 {
        &self.0.get_active().unwrap().swap_chain
    }
    pub fn draw(
        &mut self,
        env: &mut DCompBackend,
        f: impl FnMut(&Canvas),
    ) -> windows::core::Result<()> {
        self.0
            .get_active_mut()
            .unwrap()
            .draw(&mut env.d3d12, f)
            .ok()
    }
    pub fn unwrap_surface(&mut self, env: &mut DCompBackend) -> &mut Surface {
        self.0.get_active_mut().unwrap().current_surface()
    }
    pub fn present(&mut self, env: &mut DCompBackend) -> windows::core::Result<()> {
        if let Some(swap_chain) = self.0.get_active_mut() {
            swap_chain.flush_and_present(&mut env.d3d12).ok()?;
        }
        Ok(())
    }
}
