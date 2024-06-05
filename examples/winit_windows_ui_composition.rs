use skia_d3d12_swap_chain::{
    D3d12Backend, WindowsUiCompositionBackend, WindowsUiCompositionTarget,
};
use skia_safe::{colors, Paint};
use windows::Foundation::Numerics::Vector2;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut composition = WindowsUiCompositionBackend::new().unwrap();
    let mut d3d12 = D3d12Backend::new().unwrap();

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let target = WindowsUiCompositionTarget::with_window(&window).unwrap();

    let mut size = window.inner_size();
    let mut swap_chain = composition
        .create_swap_chain(size.width, size.height)
        .unwrap();

    let visual = target.create_visual(&swap_chain).unwrap();
    visual
        .SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 })
        .unwrap();

    target.desktop_window_target.SetRoot(&visual).unwrap();

    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
        .run(move |event, elwt| match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                swap_chain
                    .draw(&mut d3d12, |canvas| {
                        canvas.clear(colors::BLACK);

                        canvas.draw_circle(
                            ((size.width / 2) as i32, (size.height / 2) as i32),
                            size.width.min(size.height) as f32 / 2.0,
                            &Paint::new(colors::CYAN, None),
                        );
                    })
                    .ok()
                    .unwrap();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                swap_chain
                    .resize(&mut d3d12, new_size.width, new_size.height)
                    .unwrap();
                size = new_size;
            }
            _ => (),
        })
        .unwrap();
}
