use crate::swap_chain::{
    swap_chain_desc_composition, swap_chain_desc_hwnd, SkiaD3d12SwapChain,
    SkiaD3d12SwapChainSurfaceArray,
};
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
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D::D3D_FEATURE_LEVEL_11_0,
            Direct3D12::{D3D12CreateDevice, D3D12_RESOURCE_STATE_COMMON},
            Dxgi::{
                Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_STANDARD_MULTISAMPLE_QUALITY_PATTERN},
                CreateDXGIFactory1, IDXGIFactory4, IDXGISwapChain3, DXGI_ADAPTER_FLAG,
                DXGI_ADAPTER_FLAG_NONE, DXGI_ADAPTER_FLAG_SOFTWARE,
            },
        },
    },
};

pub struct D3d12Backend {
    factory: IDXGIFactory4,
    backend_context: BackendContext,
    direct_context: DirectContext,
}
impl D3d12Backend {
    pub fn new() -> windows::core::Result<Self> {
        let factory: IDXGIFactory4 = unsafe { CreateDXGIFactory1() }?;
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
            factory,
            backend_context,
            direct_context,
        })
    }
    pub fn create_window_swap_chain<W: HasRawWindowHandle>(
        &mut self,
        window: &W,
        width: u32,
        height: u32,
    ) -> windows::core::Result<SkiaD3d12SwapChain> {
        self.create_raw_window_handle_swap_chain(window.raw_window_handle(), width, height)
    }
    pub fn create_raw_window_handle_swap_chain(
        &mut self,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
    ) -> windows::core::Result<SkiaD3d12SwapChain> {
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
    ) -> windows::core::Result<SkiaD3d12SwapChain> {
        let swap_chain: IDXGISwapChain3 = unsafe {
            self.factory.CreateSwapChainForHwnd(
                &self.backend_context.queue,
                hwnd,
                &swap_chain_desc_hwnd(width, height),
                None,
                None,
            )
        }?
        .cast()?;

        let surfaces = self.create_swap_chain_surfaces(&swap_chain, width, height);

        Ok(SkiaD3d12SwapChain::new(swap_chain, surfaces))
    }
    pub fn create_composition_swap_chain(
        &mut self,
        width: u32,
        height: u32,
    ) -> windows::core::Result<SkiaD3d12SwapChain> {
        let swap_chain: IDXGISwapChain3 = unsafe {
            self.factory.CreateSwapChainForComposition(
                &self.backend_context.queue,
                &swap_chain_desc_composition(width, height),
                None,
            )
        }?
        .cast()?;

        let surfaces = self.create_swap_chain_surfaces(&swap_chain, width, height);

        Ok(SkiaD3d12SwapChain::new(swap_chain, surfaces))
    }
    pub(crate) fn create_swap_chain_surfaces(
        &mut self,
        swap_chain: &IDXGISwapChain3,
        width: u32,
        height: u32,
    ) -> SkiaD3d12SwapChainSurfaceArray {
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
                &mut self.direct_context,
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
        self.direct_context
            .flush_and_submit_surface(surface, sync_cpu);
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
