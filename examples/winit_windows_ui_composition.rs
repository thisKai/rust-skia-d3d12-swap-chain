use skia_d3d12_swap_chain::{CompositionBackend, CompositionTarget};
use skia_safe::{colors, Canvas, Paint};
use windows::Foundation::Numerics::Vector2;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowBuilderExtWindows,
    window::WindowBuilder,
};
fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut composition = CompositionBackend::new().unwrap();

    let window = WindowBuilder::new()
        .with_transparent(true)
        .with_no_redirection_bitmap(true)
        .build(&event_loop)
        .unwrap();
    let target = CompositionTarget::with_window(&window).unwrap();

    let mut size = window.inner_size();
    let mut swap_chain = composition
        .create_swap_chain(size.width, size.height)
        .unwrap();

    let surface = target.create_surface(&swap_chain).unwrap().unwrap();
    let brush = target
        .compositor
        .CreateSurfaceBrushWithSurface(&surface)
        .unwrap();
    brush
        .SetStretch(windows::UI::Composition::CompositionStretch::UniformToFill)
        .unwrap();

    let visual = target.compositor.CreateSpriteVisual().unwrap();
    visual.SetBrush(&brush).unwrap();
    visual
        .SetRelativeSizeAdjustment(Vector2 { X: 1.0, Y: 1.0 })
        .unwrap();

    target.desktop_window_target.SetRoot(&visual).unwrap();

    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
        .run(move |event, elwt| match event {
            Event::Resumed => {
                swap_chain
                    .draw(&mut composition, |canvas| draw(canvas, size))
                    .unwrap();
            }
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
                if let Some(new_surface) = swap_chain
                    .new_composition_surface(&mut composition, &target)
                    .unwrap()
                {
                    brush.SetSurface(&new_surface).unwrap();
                }

                swap_chain
                    .draw(&mut composition, |canvas| draw(canvas, size))
                    .unwrap();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                swap_chain.resize(&mut composition, new_size.width, new_size.height);
                size = new_size;

                if let Some(new_surface) = swap_chain
                    .new_composition_surface(&mut composition, &target)
                    .unwrap()
                {
                    brush.SetSurface(&new_surface).unwrap();
                }
            }
            _ => (),
        })
        .unwrap();
}

fn draw(canvas: &Canvas, size: PhysicalSize<u32>) {
    canvas.clear(colors::BLACK);

    canvas.draw_circle(
        ((size.width / 2) as i32, (size.height / 2) as i32),
        size.width.min(size.height) as f32 / 2.0,
        &Paint::new(colors::CYAN, None),
    );
}
