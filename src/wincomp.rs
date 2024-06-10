use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use skia_safe::{Canvas, Surface};
use windows::{
    core::Interface,
    System::DispatcherQueueController,
    Win32::{
        Foundation::HWND,
        Graphics::Dxgi::DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
        System::WinRT::{
            Composition::{ICompositorDesktopInterop, ICompositorInterop},
            CreateDispatcherQueueController, DispatcherQueueOptions,
            DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DISPATCHERQUEUE_THREAD_TYPE, DQTAT_COM_NONE,
            DQTYPE_THREAD_CURRENT,
        },
    },
    UI::Composition::{
        Core::CompositorController, Desktop::DesktopWindowTarget, ICompositionSurface,
    },
};

use crate::d3d12::{
    swap_chain::{SwapChain, SwapChainState},
    Backend,
};

pub struct WinCompBackend {
    _dispatcher_queue_controller: DispatcherQueueController,
    d3d12: Backend,
}
impl WinCompBackend {
    pub fn new() -> windows::core::Result<Self> {
        Ok(Self {
            _dispatcher_queue_controller: create_dispatcher_queue_controller_for_current_thread()?,
            d3d12: Backend::new()?,
        })
    }
    pub fn create_swap_chain(
        &mut self,
        width: u32,
        height: u32,
    ) -> windows::core::Result<WinCompSwapChain> {
        Ok(WinCompSwapChain::new(
            self.d3d12.create_swap_chain_for_composition(
                width,
                height,
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
            )?,
        ))
    }
}

pub struct WinCompTarget {
    pub controller: CompositorController,
    pub desktop_window_target: DesktopWindowTarget,
}
impl WinCompTarget {
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
        let controller = CompositorController::new()?;
        let compositor = controller.Compositor()?;
        let compositor_desktop_interop: ICompositorDesktopInterop = compositor.cast()?;
        let desktop_window_target =
            unsafe { compositor_desktop_interop.CreateDesktopWindowTarget(hwnd, true) }?;

        Ok(Self {
            controller,
            desktop_window_target,
        })
    }
    pub fn create_surface(
        &self,
        swap_chain: &WinCompSwapChain,
    ) -> windows::core::Result<Option<ICompositionSurface>> {
        swap_chain
            .swap_chain
            .get_active()
            .map(|swap_chain| self.create_surface_internal(swap_chain))
            .transpose()
    }
    fn create_surface_internal(
        &self,
        swap_chain: &SwapChain,
    ) -> windows::core::Result<ICompositionSurface> {
        let compositor_interop: ICompositorInterop = self.controller.Compositor()?.cast()?;

        unsafe { compositor_interop.CreateCompositionSurfaceForSwapChain(&swap_chain.swap_chain) }
    }
}

pub struct WinCompSwapChain {
    swap_chain: SwapChainState,
    needs_dwm_flush: bool,
}
impl WinCompSwapChain {
    fn new(swap_chain: SwapChain) -> Self {
        Self {
            swap_chain: SwapChainState::Active(swap_chain),
            needs_dwm_flush: false,
        }
    }
    pub fn resize(
        &mut self,
        env: &mut WinCompBackend,
        target: &WinCompTarget,
        width: u32,
        height: u32,
    ) {
        self.swap_chain.resize(
            &mut env.d3d12,
            width,
            height,
            DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
        );
        self.needs_dwm_flush = true;
    }
    pub fn new_composition_surface(
        &mut self,
        env: &mut WinCompBackend,
        target: &WinCompTarget,
    ) -> windows::core::Result<Option<ICompositionSurface>> {
        if let Some((width, height)) = self.swap_chain.needs_resize() {
            env.d3d12.recreate_context_if_needed()?;

            let swap_chain = env.d3d12.create_swap_chain_for_composition(
                width,
                height,
                DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT,
            )?;
            let surface = target.create_surface_internal(&swap_chain)?;

            self.swap_chain = SwapChainState::Active(swap_chain);

            Ok(Some(surface))
        } else {
            Ok(None)
        }
    }
    pub fn draw(
        &mut self,
        env: &mut WinCompBackend,
        f: impl FnMut(&Canvas),
    ) -> windows::core::Result<()> {
        self.swap_chain
            .get_active_mut()
            .unwrap()
            .draw(&mut env.d3d12, f)
            .ok()
    }
    pub fn unwrap_surface_mut(&mut self) -> &mut Surface {
        self.swap_chain.get_active_mut().unwrap().current_surface()
    }
    pub fn present(
        &mut self,
        env: &mut WinCompBackend,
        target: &WinCompTarget,
    ) -> windows::core::Result<()> {
        if let Some(swap_chain) = self.swap_chain.get_active_mut() {
            swap_chain.wait()?;

            swap_chain.flush_and_present(&mut env.d3d12).ok()?;

            target.controller.Commit()?;
        }
        Ok(())
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
