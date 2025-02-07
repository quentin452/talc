mod bevy_winit_event_converters;
use bevy_app::PluginsState;
use bevy_input::keyboard::KeyboardInput;
use bevy_winit_event_converters::*;

use std::{ops::Deref, sync::atomic::{AtomicBool, Ordering}};

use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::ActiveEventLoop, window::{CursorGrabMode, Window, WindowId}};

use crate::{add_plugins, bevy::prelude::*, render::wgpu_context::{RenderDevice, WgpuContext}};

#[derive(Resource)]
pub struct PrimaryWindow {
    inner: &'static Window,
    is_cursor_locked: AtomicBool
}

impl Deref for PrimaryWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl PrimaryWindow {
    fn new(window: &'static Window) -> Self {
        Self {
            inner: window,
            is_cursor_locked: AtomicBool::new(false)
        }
    }

    pub fn width(&self) -> u32 {
        self.inner_size().width
    }

    pub fn height(&self) -> u32 {
        self.inner_size().height
    }

    pub fn lock_cursor(&self) {
        self.set_cursor_visible(false);
        self
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| self.set_cursor_grab(CursorGrabMode::Confined))
            .expect("Failed to grab cursor");
        self.is_cursor_locked.store(true, Ordering::Relaxed);
    }

    #[allow(dead_code)]
    pub fn unlock_cursor(&self) {
        self.set_cursor_visible(true);
        self
            .set_cursor_grab(CursorGrabMode::None)
            .expect("Failed to release cursor");
        self.is_cursor_locked.store(false, Ordering::Relaxed);
    }

    /// Grabs/ungrabs mouse cursor
    pub fn toggle_grab_cursor(&self) {
        if self.is_cursor_locked() {
            self.unlock_cursor();
        } else {
            self.lock_cursor();
        }
    }

    #[inline]
    pub fn is_cursor_locked(&self) -> bool {
        self.is_cursor_locked.load(Ordering::Relaxed)
    }
}

pub struct Winit {
    app: App,
    window: Option<&'static Window>,
    bevy_window_events: Vec<KeyboardInput>
}

impl Winit {
    pub fn new(app: App) -> Self {
        Self {
            app,
            window: None,
            bevy_window_events: vec![]
        }
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
        let render_device = RenderDevice(wgpu_context.device.clone());
        let primary_window = PrimaryWindow::new(window);

        self.app.world_mut().insert_resource(wgpu_context);
        self.app.world_mut().insert_resource(render_device);
        self.app.world_mut().insert_resource(primary_window);

        add_plugins(&mut self.app);
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
                //self.render();
            }
            WindowEvent::KeyboardInput {
                ref event,
                // On some platforms, winit sends "synthetic" key press events when the window
                // gains or loses focus. These should not be handled, so we only process key
                // events if they are not synthetic key presses.
                is_synthetic: false,
                ..
            } => {
                self.bevy_window_events.push(convert_keyboard_input(event));
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        self.run_app_update();
    }
}

impl Winit {
    fn run_app_update(&mut self) {
        self.forward_bevy_events();

        if self.app.plugins_state() == PluginsState::Cleaned {
            self.app.update();
        }
    }
    
    fn forward_bevy_events(&mut self) {
        let buffered_events = self.bevy_window_events.drain(..).collect::<Vec<_>>();
        let world = self.app.world_mut();

        for winit_event in buffered_events.into_iter() {
            world.send_event(winit_event).expect("Failed to execute keyboard event");
        }
    }
}