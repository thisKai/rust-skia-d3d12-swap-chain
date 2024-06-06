use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use skia_safe::{Canvas, Surface};
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
    UI::Composition::{Compositor, Desktop::DesktopWindowTarget, ICompositionSurface},
};

use crate::d3d12::{
    swap_chain::{SwapChain, SwapChainState},
    Backend,
};

pub struct CompositionBackend {
    _dispatcher_queue_controller: DispatcherQueueController,
    d3d12: Backend,
}
impl CompositionBackend {
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
    ) -> windows::core::Result<CompositionSwapChain> {
        Ok(CompositionSwapChain::new(
            self.d3d12
                .create_swap_chain_for_composition(width, height)?,
        ))
    }
}

pub struct CompositionTarget {
    pub compositor: Compositor,
    pub desktop_window_target: DesktopWindowTarget,
}
impl CompositionTarget {
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
    pub fn create_surface(
        &self,
        swap_chain: &CompositionSwapChain,
    ) -> windows::core::Result<Option<ICompositionSurface>> {
        swap_chain
            .0
            .get_active()
            .map(|swap_chain| self.create_surface_internal(swap_chain))
            .transpose()
    }
    fn create_surface_internal(
        &self,
        swap_chain: &SwapChain,
    ) -> windows::core::Result<ICompositionSurface> {
        let compositor_interop: ICompositorInterop = self.compositor.cast()?;

        unsafe { compositor_interop.CreateCompositionSurfaceForSwapChain(&swap_chain.swap_chain) }
    }
}

pub struct CompositionSwapChain(SwapChainState);
impl CompositionSwapChain {
    fn new(swap_chain: SwapChain) -> Self {
        Self(SwapChainState::Active(swap_chain))
    }
    pub fn resize(&mut self, env: &mut CompositionBackend, width: u32, height: u32) {
        self.0.resize(&mut env.d3d12, width, height);
    }
    pub fn new_composition_surface(
        &mut self,
        env: &mut CompositionBackend,
        target: &CompositionTarget,
    ) -> windows::core::Result<Option<ICompositionSurface>> {
        if let Some((width, height)) = self.0.needs_resize() {
            env.d3d12.recreate_context_if_needed()?;

            let swap_chain = env.d3d12.create_swap_chain_for_composition(width, height)?;
            let surface = target.create_surface_internal(&swap_chain)?;

            self.0 = SwapChainState::Active(swap_chain);

            Ok(Some(surface))
        } else {
            Ok(None)
        }
    }
    pub fn draw(
        &mut self,
        env: &mut CompositionBackend,
        f: impl FnMut(&Canvas),
    ) -> windows::core::Result<()> {
        self.0
            .get_active_mut()
            .unwrap()
            .draw(&mut env.d3d12, f)
            .ok()
    }
    pub fn unwrap_surface(&mut self, env: &mut CompositionBackend) -> &mut Surface {
        self.0.get_active_mut().unwrap().get_surface()
    }
    pub fn present(&mut self, env: &mut CompositionBackend) {
        if let Some(swap_chain) = self.0.get_active_mut() {
            swap_chain.present(&mut env.d3d12);
        }
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
