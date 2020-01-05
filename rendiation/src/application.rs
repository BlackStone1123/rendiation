use winit::event::WindowEvent;
use crate::renderer::*;

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::mem::size_of;
    use std::slice::from_raw_parts;

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

pub trait Application: 'static + Sized {
    fn init(
        renderer: &WGPURenderer
    ) -> (Self, Option<wgpu::CommandBuffer>);
    fn resize(
        &mut self,
        renderer: &WGPURenderer
    ) -> Option<wgpu::CommandBuffer>;
    fn update(&mut self, event: WindowEvent);
    fn render(
        &mut self,
        frame: &wgpu::SwapChainOutput,
        device: &wgpu::Device,
    ) -> wgpu::CommandBuffer;
}

pub fn run<E: Application>(title: &str) {
    use winit::{
        event,
        event_loop::{ControlFlow, EventLoop},
    };

    let event_loop = EventLoop::new();
    log::info!("Initializing the window...");

    #[cfg(not(feature = "gl"))]
    let (_window, hidpi_factor, size, surface) = {
        let window = winit::window::Window::new(&event_loop).unwrap();
        window.set_title(title);
        let hidpi_factor = window.hidpi_factor();
        let size = window.inner_size().to_physical(hidpi_factor);
        let surface = wgpu::Surface::create(&window);
        (window, hidpi_factor, size, surface)
    };

    #[cfg(feature = "gl")]
    let (_window, instance, hidpi_factor, size, surface) = {
        let wb = winit::WindowBuilder::new();
        let cb = wgpu::glutin::ContextBuilder::new().with_vsync(true);
        let context = cb.build_windowed(wb, &event_loop).unwrap();
        context.window().set_title(title);

        let hidpi_factor = context.window().hidpi_factor();
        let size = context
            .window()
            .get_inner_size()
            .unwrap()
            .to_physical(hidpi_factor);

        let (context, window) = unsafe { context.make_current().unwrap().split() };

        let instance = wgpu::Instance::new(context);
        let surface = instance.get_surface();

        (window, instance, hidpi_factor, size, surface)
    };

    // let adapter = wgpu::Adapter::request(&wgpu::RequestAdapterOptions {
    //     power_preference: wgpu::PowerPreference::Default,
    //     backends: wgpu::BackendBit::PRIMARY,
    // })
    // .unwrap();

    // let (device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
    //     extensions: wgpu::Extensions {
    //         anisotropic_filtering: false,
    //     },
    //     limits: wgpu::Limits::default(),
    // });

    // let mut sc_desc = wgpu::SwapChainDescriptor {
    //     usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    //     format: wgpu::TextureFormat::Bgra8UnormSrgb,
    //     width: size.width.round() as u32,
    //     height: size.height.round() as u32,
    //     present_mode: wgpu::PresentMode::Vsync,
    // };
    // let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);
    let mut renderer = WGPURenderer::new(surface, (size.width.round() as usize, size.height.round() as usize));

    log::info!("Initializing the example...");
    let (mut example, init_command_buf) = E::init(&renderer);
    if let Some(command_buf) = init_command_buf {
        renderer.queue.submit(&[command_buf]);
    }

    log::info!("Entering render loop...");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if cfg!(feature = "metal-auto-capture") {
            ControlFlow::Exit
        } else {
            ControlFlow::Poll
        };
        match event {
            event::Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let physical = size.to_physical(hidpi_factor);
                log::info!("Resizing to {:?}", physical);
                renderer.resize(physical.width.round() as usize, physical.height.round() as usize);
                let command_buf = example.resize(&renderer);
                if let Some(command_buf) = command_buf {
                    renderer.queue.submit(&[command_buf]);
                }
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {
                    example.update(event);
                }
            },
            event::Event::EventsCleared => {
                let frame = renderer.swap_chain.get_next_texture();
                let command_buf = example.render(&frame, &renderer.device);
                renderer.queue.submit(&[command_buf]);
            }
            _ => (),
        }
    });
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
#[allow(dead_code)]
fn main() {}
