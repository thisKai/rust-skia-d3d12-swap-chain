use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use windows::{
    core::Interface,
    System::DispatcherQueueController,
    Win32::{
        Foundation::HWND,
        System::WinRT::{
            Composition::{ICompositorDesktopInterop, ICompositorInterop},
            CreateDispatcherQueueController, DispatcherQueueOptions,
            DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DISPATCHERQUEUE_THREAD_TYPE, DQTAT_COM_NONE,
            DQTYPE_THREAD_CURRENT,
        },
    },
    UI::Composition::{
        CompositionSurfaceBrush, Compositor, Desktop::DesktopWindowTarget, ICompositionSurface,
        SpriteVisual,
    },
};

use crate::{d3d12::D3d12Backend, SkiaD3d12SwapChain};

pub struct WindowsUiCompositionBackend {
    _dispatcher_queue_controller: DispatcherQueueController,
    d3d12: D3d12Backend,
}
impl WindowsUiCompositionBackend {
    pub fn new() -> windows::core::Result<Self> {
        Ok(Self {
            _dispatcher_queue_controller: create_dispatcher_queue_controller_for_current_thread()?,
            d3d12: D3d12Backend::new()?,
        })
    }
    pub fn create_swap_chain(
        &mut self,
        width: u32,
        height: u32,
    ) -> windows::core::Result<SkiaD3d12SwapChain> {
        self.d3d12.create_composition_swap_chain(width, height)
    }
}

pub struct WindowsUiCompositionTarget {
    pub compositor: Compositor,
    pub desktop_window_target: DesktopWindowTarget,
}
impl WindowsUiCompositionTarget {
    pub fn with_window<W: HasRawWindowHandle>(window: &W) -> windows::core::Result<Self> {
        Self::with_raw_window_handle(window.raw_window_handle())
    }
    pub fn with_raw_window_handle(window_handle: RawWindowHandle) -> windows::core::Result<Self> {
        let hwnd = match window_handle {
            RawWindowHandle::Win32(window_handle) => HWND(window_handle.hwnd as _),
            _ => panic!("not win32"),
        };
        Self::with_hwnd(hwnd)
    }
    pub fn with_hwnd(hwnd: HWND) -> windows::core::Result<Self> {
        let compositor = Compositor::new()?;
        let compositor_desktop_interop: ICompositorDesktopInterop = compositor.cast()?;
        let desktop_window_target =
            unsafe { compositor_desktop_interop.CreateDesktopWindowTarget(hwnd, true) }?;

        Ok(Self {
            compositor,
            desktop_window_target,
        })
    }
    pub fn create_visual(
        &self,
        swap_chain: &SkiaD3d12SwapChain,
    ) -> windows::core::Result<SpriteVisual> {
        let visual = self.compositor.CreateSpriteVisual()?;

        let brush = self.create_brush(swap_chain)?;
        visual.SetBrush(&brush)?;

        Ok(visual)
    }
    pub fn create_brush(
        &self,
        swap_chain: &SkiaD3d12SwapChain,
    ) -> windows::core::Result<CompositionSurfaceBrush> {
        let surface = self.create_surface(swap_chain)?;

        self.compositor.CreateSurfaceBrushWithSurface(&surface)
    }
    pub fn create_surface(
        &self,
        swap_chain: &SkiaD3d12SwapChain,
    ) -> windows::core::Result<ICompositionSurface> {
        let compositor_interop: ICompositorInterop = self.compositor.cast()?;

        unsafe { compositor_interop.CreateCompositionSurfaceForSwapChain(&swap_chain.swap_chain) }
    }
}

pub(crate) fn create_dispatcher_queue_controller_for_current_thread(
) -> windows::core::Result<DispatcherQueueController> {
    create_dispatcher_queue_controller(DQTYPE_THREAD_CURRENT, DQTAT_COM_NONE)
}

fn create_dispatcher_queue_controller(
    thread_type: DISPATCHERQUEUE_THREAD_TYPE,
    apartment_type: DISPATCHERQUEUE_THREAD_APARTMENTTYPE,
) -> windows::core::Result<DispatcherQueueController> {
    let options = DispatcherQueueOptions {
        dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
        threadType: thread_type,
        apartmentType: apartment_type,
    };
    unsafe { CreateDispatcherQueueController(options) }
}
