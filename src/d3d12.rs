pub(crate) mod swap_chain;

use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use skia_safe::{
    gpu::{
        d3d::{
            BackendContext, ID3D12CommandQueue, ID3D12Device, IDXGIAdapter1, TextureResourceInfo,
        },
        surfaces, BackendRenderTarget, DirectContext, Protected, SurfaceOrigin, SyncCpu,
    },
    ColorType, Surface,
};
use swap_chain::{
    swap_chain_desc_composition, swap_chain_desc_hwnd, HwndSwapChain, SwapChain,
    SwapChainSurfaceArray,
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{D3D12CreateDevice, D3D12_RESOURCE_STATE_COMMON},
            Dwm::DwmFlush,
            Dxgi::{
                Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN},
                CreateDXGIFactory1, IDXGIFactory4, IDXGISwapChain3, DXGI_ADAPTER_FLAG,
                DXGI_ADAPTER_FLAG_NONE, DXGI_ADAPTER_FLAG_SOFTWARE,
            },
        },
    },
};

pub struct Backend {
    factory: IDXGIFactory4,
    skia_context: OptionalSkiaContext,
}
impl Backend {
    pub fn new() -> windows::core::Result<Self> {
        let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1() }?;

        let skia_context = OptionalSkiaContext::new(&factory)?;

        Ok(Self {
            factory,
            skia_context,
        })
    }
    pub fn release_context(&mut self) {
        self.skia_context.release();
    }
    pub fn recreate_context_if_needed(&mut self) -> windows::core::Result<bool> {
        self.skia_context.recreate_if_needed(&self.factory)
    }
    pub fn create_window_swap_chain<W: HasRawWindowHandle>(
        &mut self,
        window: &W,
        width: u32,
        height: u32,
    ) -> windows::core::Result<HwndSwapChain> {
        self.create_raw_window_handle_swap_chain(window.raw_window_handle(), width, height)
    }
    pub fn create_raw_window_handle_swap_chain(
        &mut self,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
    ) -> windows::core::Result<HwndSwapChain> {
        let hwnd = match window_handle {
            RawWindowHandle::Win32(window_handle) => HWND(window_handle.hwnd as _),
            _ => panic!("not win32"),
        };
        self.create_hwnd_swap_chain(hwnd, width, height)
    }
    pub fn create_hwnd_swap_chain(
        &mut self,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> windows::core::Result<HwndSwapChain> {
        Ok(HwndSwapChain::new(
            hwnd,
            self.create_swap_chain_for_hwnd(hwnd, width, height)?,
        ))
    }
    pub fn create_swap_chain_for_hwnd(
        &mut self,
        hwnd: HWND,
        width: u32,
        height: u32,
    ) -> windows::core::Result<SwapChain> {
        let swap_chain: IDXGISwapChain3 = unsafe {
            self.factory.CreateSwapChainForHwnd(
                &self.skia_context.unwrap_ref().backend_context.queue,
                hwnd,
                &swap_chain_desc_hwnd(width, height),
                None,
                None,
            )
        }?
        .cast()?;

        let surfaces = self.create_swap_chain_surfaces(&swap_chain, width, height);

        Ok(SwapChain::new(swap_chain, surfaces))
    }
    pub fn create_swap_chain_for_composition(
        &mut self,
        width: u32,
        height: u32,
    ) -> windows::core::Result<SwapChain> {
        let swap_chain: IDXGISwapChain3 = unsafe {
            self.factory.CreateSwapChainForComposition(
                &self.skia_context.unwrap_ref().backend_context.queue,
                &swap_chain_desc_composition(width, height),
                None,
            )
        }?
        .cast()?;

        let surfaces = self.create_swap_chain_surfaces(&swap_chain, width, height);

        Ok(SwapChain::new(swap_chain, surfaces))
    }
    pub(crate) fn create_swap_chain_surfaces(
        &mut self,
        swap_chain: &IDXGISwapChain3,
        width: u32,
        height: u32,
    ) -> SwapChainSurfaceArray {
        std::array::from_fn(|i| {
            let resource = unsafe { swap_chain.GetBuffer(i as u32).unwrap() };

            let backend_render_target = BackendRenderTarget::new_d3d(
                (width.try_into().unwrap(), height.try_into().unwrap()),
                &TextureResourceInfo {
                    resource,
                    alloc: None,
                    resource_state: D3D12_RESOURCE_STATE_COMMON,
                    format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    sample_count: 1,
                    level_count: 0,
                    sample_quality_pattern: DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN,
                    protected: Protected::No,
                },
            );

            let surface = surfaces::wrap_backend_render_target(
                &mut self.skia_context.unwrap_mut().direct_context,
                &backend_render_target,
                SurfaceOrigin::TopLeft,
                ColorType::RGBA8888,
                None,
                None,
            )
            .unwrap();

            (surface, backend_render_target)
        })
    }
    pub(crate) fn flush_and_submit_surface(
        &mut self,
        surface: &mut Surface,
        sync_cpu: impl Into<Option<SyncCpu>>,
    ) {
        self.skia_context
            .unwrap_mut()
            .flush_and_submit_surface(surface, sync_cpu)
    }
    pub fn get_device_removed_reason(&self) -> windows::core::Result<()> {
        self.skia_context.unwrap_ref().get_device_removed_reason()
    }
    pub fn cleanup(&mut self) {
        self.skia_context.unwrap_mut().cleanup()
    }
    pub fn dwm_flush(&self) -> windows::core::Result<()> {
        unsafe { DwmFlush() }
    }
}

struct OptionalSkiaContext(Option<SkiaContext>);
impl OptionalSkiaContext {
    fn new(factory: &IDXGIFactory4) -> windows::core::Result<Self> {
        Ok(Self(Some(SkiaContext::new(factory)?)))
    }
    fn recreate_if_needed(&mut self, factory: &IDXGIFactory4) -> windows::core::Result<bool> {
        if self.0.is_none() {
            self.0 = Some(SkiaContext::new(factory)?);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    fn unwrap_ref(&self) -> &SkiaContext {
        self.0.as_ref().unwrap()
    }
    fn unwrap_mut(&mut self) -> &mut SkiaContext {
        self.0.as_mut().unwrap()
    }
    fn release(&mut self) {
        self.0 = None;
    }
}

struct SkiaContext {
    backend_context: BackendContext,
    direct_context: DirectContext,
}
impl SkiaContext {
    fn new(factory: &IDXGIFactory4) -> windows::core::Result<Self> {
        let (adapter, device) = get_hardware_adapter_and_device(&factory)?;
        let queue: ID3D12CommandQueue = unsafe { device.CreateCommandQueue(&Default::default()) }?;

        let backend_context = BackendContext {
            adapter,
            device,
            queue,
            memory_allocator: None,
            protected_context: Protected::No,
        };
        let direct_context = unsafe { DirectContext::new_d3d(&backend_context, None) }.unwrap();

        Ok(Self {
            backend_context,
            direct_context,
        })
    }
    pub(crate) fn flush_and_submit_surface(
        &mut self,
        surface: &mut Surface,
        sync_cpu: impl Into<Option<SyncCpu>>,
    ) {
        self.direct_context
            .flush_and_submit_surface(surface, sync_cpu);
    }
    pub fn get_device_removed_reason(&self) -> windows::core::Result<()> {
        unsafe { self.backend_context.device.GetDeviceRemovedReason() }
    }
    pub fn cleanup(&mut self) {
        self.direct_context
            .perform_deferred_cleanup(Default::default(), None);
    }
}

fn get_hardware_adapter_and_device(
    factory: &IDXGIFactory4,
) -> windows::core::Result<(IDXGIAdapter1, ID3D12Device)> {
    for i in 0.. {
        let adapter = unsafe { factory.EnumAdapters1(i) }?;

        let mut adapter_desc = Default::default();
        unsafe { adapter.GetDesc1(&mut adapter_desc) }?;

        if (DXGI_ADAPTER_FLAG(adapter_desc.Flags as _) & DXGI_ADAPTER_FLAG_SOFTWARE)
            != DXGI_ADAPTER_FLAG_NONE
        {
            continue; // Don't select the Basic Render Driver adapter.
        }

        let mut device = None;
        if unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, &mut device) }.is_ok() {
            return Ok((adapter, device.unwrap()));
        }
    }
    unreachable!()
}
