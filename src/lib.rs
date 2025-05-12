
pub mod launcher;
pub mod render;
pub mod world;

use render::renderer::Renderer;
use world::Scene;
use tokio::runtime::Runtime;
use winit::{
        event_loop::EventLoopWindowTarget,
        event::{WindowEvent, DeviceEvent, KeyEvent, ElementState},
        keyboard::{PhysicalKey, KeyCode},
        window::Window
    };


#[derive(PartialEq)]
pub enum GameState {

    PLAYING,
    PAUSED
}




pub struct Engine<'a> {
    pub window: &'a Window,
    renderer: Renderer<'a>,
    scene: Scene,
    state: GameState

}

impl<'a> Engine<'a> {

    pub async fn new(window: &'a Window, runtime: Runtime) -> Self {

        let mut renderer = Renderer::new(&window, &runtime).await;

        let scene = Scene::new(&mut renderer);

        Self {
            window,
            renderer,
            scene,
            state: GameState::PLAYING,
        }
    }

    //TODO: add global settings as parameter
    pub fn handle_window_event(&mut self, event: WindowEvent, elwt: &EventLoopWindowTarget<()>) {
        if !self.scene.handle_input_event(&event, &self.state) {
        match event {
            WindowEvent::CloseRequested  => {
                elwt.exit()
            },

            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
            }, 
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let dt = now - self.renderer.last_render_time;
                self.renderer.last_render_time = now;
                self.update(dt);
                match self.renderer.render(&self.scene.terrain, &self.scene.globals_bind_group) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.renderer.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e)
                }
                
            },
            // WindowEvent::MouseWheel { delta, .. } => {
            //     self.scene.camera.camera_controller.process_scroll(&delta);
            // },
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key:PhysicalKey::Code(KeyCode::Escape),
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => {
                self.state = match self.state {
                    GameState::PAUSED =>
                    {
                        // in windows os this doesnt work
                        // self.window.set_cursor_grab(winit::window::CursorGrabMode::Locked).unwrap();
                        self.window.set_cursor_visible(false);
                        GameState::PLAYING
                    },
                    GameState::PLAYING =>
                    {
                        let center = winit::dpi::PhysicalPosition::new(self.renderer.size.width / 2, self.renderer.size.height / 2);
                        self.window.set_cursor_position(center).unwrap_or_else(|e| {
                            eprintln!("Failed to set cursor position: {:?}", e);
                        });
                        self.window.set_cursor_grab(winit::window::CursorGrabMode::None).unwrap();
                        self.window.set_cursor_visible(true);

                        
                        GameState::PAUSED
                    },
                    
                }
            }
            
            _ => {}
        }

            
        }

    }


    pub fn initialize(&mut self) {
        self.window.set_cursor_visible(false);
        // in windows os this doesnt work
        // self.window.set_cursor_grab(winit::window::CursorGrabMode::Locked).unwrap();
        let center = winit::dpi::PhysicalPosition::new(self.renderer.size.width / 2, self.renderer.size.height / 2);
        self.window.set_cursor_position(center).unwrap_or_else(|e| {
            eprintln!("Failed to set cursor position: {:?}", e);
        });


    }



    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.scene.camera.resize(new_size);
        self.renderer.resize(new_size);

        
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        self.scene.update(&mut self.renderer, dt);
        self.renderer.update()
    }

    pub fn handle_device_input(&mut self, event: &DeviceEvent, _: &EventLoopWindowTarget<()>) {

        if self.state == GameState::PLAYING {
            self.scene.camera.input(event);
        }
    }

    
}