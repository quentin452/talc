use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, ControlFlow, EventLoop}, window::{Window, WindowId}};

use crate::{bevy::prelude::*, render::wgpu_context::WgpuContext};

pub struct Winit {
    app: App,
    window: Option<&'static Window>
}

impl Winit {
    pub fn run(mut app: App) {
        let event_loop = EventLoop::new().expect("Failed to create winit event loop.");
        event_loop.set_control_flow(ControlFlow::Poll);
        app.run();
        event_loop.run_app(&mut Self {
            app,
            window: None
        });
    }

    fn update(&mut self) {
        
    }

    fn render(&mut self) {
        
    }
}

impl ApplicationHandler for Winit {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window_attributes = Window::default_attributes().with_title("Talc");
        let window: &'static Window = Box::leak(Box::new(
            event_loop
                .create_window(window_attributes)
                .expect("create window err."),
        ));
        let wgpu_context = WgpuContext::new(window);
        self.app.world_mut().insert_resource(wgpu_context);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        _event: winit::event::DeviceEvent,
    ) {
        
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                let mut wgpu_context = self.app.world_mut().get_resource_mut::<WgpuContext>();
                if let Some(wgpu_context) = wgpu_context.as_mut() {
                    wgpu_context.resize((new_size.width, new_size.height));
                }

                if let Some(window) = self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}