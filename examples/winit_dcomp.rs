use skia_d3d12_swap_chain::DCompBackend;
use skia_safe::{colors, Canvas, Paint};
use windows::Win32::Graphics::DirectComposition::DCOMPOSITION_BACKFACE_VISIBILITY_HIDDEN;
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowBuilderExtWindows,
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut dcomp = DCompBackend::new().unwrap();

    let window = WindowBuilder::new()
        .with_transparent(true)
        .with_no_redirection_bitmap(true)
        .build(&event_loop)
        .unwrap();

    let target = dcomp.create_target_for_window(&window).unwrap();

    let mut size = window.inner_size();
    let mut swap_chain = dcomp.create_swap_chain(size.width, size.height).unwrap();
    swap_chain
        .draw(&mut dcomp, |canvas| draw(canvas, size))
        .unwrap();

    let visual = unsafe {
        let visual = dcomp.dcomp_desktop_device.CreateVisual().unwrap();
        visual
            .SetContent(swap_chain.unwrap_inner_swap_chain())
            .unwrap();
        visual
            .SetBackFaceVisibility(DCOMPOSITION_BACKFACE_VISIBILITY_HIDDEN)
            .unwrap();

        target.SetRoot(&visual).unwrap();

        dcomp.dcomp_desktop_device.Commit().unwrap();

        visual
    };

    event_loop.set_control_flow(ControlFlow::Wait);

    event_loop
        .run(move |event, elwt| match event {
            Event::Resumed => {}
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
                    .draw(&mut dcomp, |canvas| draw(canvas, size))
                    .unwrap();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                swap_chain.resize(&mut dcomp, new_size.width, new_size.height);
                size = new_size;

                if let Some(inner_swap_chain) = swap_chain.new_inner_swap_chain(&mut dcomp).unwrap()
                {
                    unsafe {
                        visual.SetContent(inner_swap_chain).unwrap();
                    }
                }
                swap_chain
                    .draw(&mut dcomp, |canvas| draw(canvas, size))
                    .unwrap();

                unsafe {
                    dcomp.dcomp_desktop_device.Commit().unwrap();
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
