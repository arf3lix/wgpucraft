
pub mod launcher;
pub mod render;
pub mod terrain_gen;
pub mod player;
pub mod ecs;
pub mod hud;



use std::time::{Duration, Instant};
use hud::HUD;
use player::{camera::Camera, raycast::Ray, Player};
use tracy::{zone, frame};

use render::{atlas::MaterialType, pipelines::{GlobalModel, Globals}, renderer::Renderer};
use terrain_gen::{biomes::PRAIRIE_PARAMS, generator::TerrainGen};
use wgpu::BindGroup;
use winit::{
        dpi::PhysicalPosition, event::{self, DeviceEvent, ElementState, KeyEvent, MouseButton, WindowEvent}, event_loop::{self, EventLoopWindowTarget}, keyboard::{KeyCode, PhysicalKey}, window::{CursorGrabMode, Window}
    };


use winit:: {
    event::Event,
};

#[derive(PartialEq)]
pub enum GameState {

    PLAYING,
    PAUSED
}




pub struct State<'a> {
    pub window: &'a Window,
    renderer: Renderer<'a>,
    pub data: GlobalModel,
    pub globals_bind_group: BindGroup,
    pub player: Player,
    pub terrain: TerrainGen,
    pub hud: HUD,
    state: GameState,
    target_frametime: Duration

}

impl<'a> State<'a> {

    pub fn new(window: &'a Window) -> Self {

        let mut renderer = Renderer::new(&window);

        let data = GlobalModel {
            globals: renderer.create_consts(&[Globals::default()]),

        };

        let hud = HUD::new(
            &renderer,
            &renderer.layouts.global,   
            renderer.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("HUD Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shaders/hud.wgsl").into()),
            }),
        );

        let globals_bind_group = renderer.bind_globals(&data);

        let camera = Camera::new(&renderer, (8.0, 12.0, 8.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));

        let player = Player::new(camera);



        let terrain = TerrainGen::new(
            &renderer,
        );

        


        Self {
            window,
            renderer,
            data,
            globals_bind_group,
            player,
            terrain,
            hud,
            state: GameState::PLAYING,
            target_frametime: Duration::from_secs_f64(1.0 / 60.0),  // 60 FPS

        }
    }

    pub fn handle_wait(&mut self, _elwt: &EventLoopWindowTarget<()>) {


        self.window.request_redraw();

    }

    //TODO: add global settings as parameter
    pub fn handle_window_event(&mut self, event: WindowEvent, elwt: &EventLoopWindowTarget<()>) {
        if !self.handle_input_event(&event) {
        match event {
            WindowEvent::CloseRequested  => {
                elwt.exit()
            },

            WindowEvent::Resized(physical_size) => {
                self.resize(physical_size);
            }, 
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let elapsed = now - self.renderer.last_render_time;
                frame!();
                zone!("redraw request"); // <- Marca el inicio del bloque


                self.renderer.last_render_time = now;
                self.update(elapsed);
                match self.renderer.render(&self.terrain, &self.hud, &self.globals_bind_group) {
                    Ok(_) => {}
                    // Reconfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => self.resize(self.renderer.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e)
                }
                
            },

            // Eventos del mouse
            WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {

                        let ray = Ray::from_camera(&self.player.camera, 100.0);
                        let ray_hit = ray.cast(&self.terrain.chunks);

                        if let Some(hit) = ray_hit {


                            if let Some(chunk_index) = self.terrain.chunks.set_block(hit.neighbor_position(), MaterialType::ROCK) {


                                let mesh = self.terrain.update_mesh(
                                    &self.terrain.chunks.blocks_array[chunk_index].read().unwrap(),
                                    PRAIRIE_PARAMS
                                );

                                *self.terrain.chunks.mesh_array[chunk_index].write().unwrap() = mesh;
                                self.terrain.chunk_models[chunk_index].update(&self.renderer.queue, &self.terrain.chunks.mesh_array[chunk_index].read().unwrap(), 0);
                            }
                            println!("Clic izquierdo presionado en: {:?}", hit.neighbor_position());
                            // Aquí puedes añadir tu lógica para el clic izquierdo
                        } else {
                            println!("No se golpeó ningún bloque");
                        }
                        



                        // Aquí puedes añadir tu lógica
                    },
                    (MouseButton::Right, ElementState::Pressed) => {
                        // Acción para clic derecho presionado
                        println!("Clic derecho presionado");
                        // Aquí puedes añadir tu lógica
                    },
                    (MouseButton::Middle, ElementState::Pressed) => {
                        // Acción para rueda del mouse presionada
                        println!("Rueda del mouse presionada");
                        // Aquí puedes añadir tu lógica
                    },
                    _ => {}
                }
            },



            // WindowEvent::MouseWheel { delta, .. } => {
            //     self.camera.camera_controller.process_scroll(&delta);
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
                        self.window.set_cursor_position(PhysicalPosition::new(self.renderer.size.width / 2, self.renderer.size.height / 2))
                            .expect("No se pudo mover el cursor");

                        // Ahora intenta bloquear el cursor
                        self.window.set_cursor_grab(CursorGrabMode::Confined)
                            .expect("No se pudo bloquear el cursor");

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
        self.window.set_cursor_position(PhysicalPosition::new(self.renderer.size.width / 2, self.renderer.size.height / 2))
            .expect("No se pudo mover el cursor");

        // Ahora intenta bloquear el cursor
        self.window.set_cursor_grab(CursorGrabMode::Confined)
            .expect("No se pudo bloquear el cursor");


        let center = winit::dpi::PhysicalPosition::new(self.renderer.size.width / 2, self.renderer.size.height / 2);
        self.window.set_cursor_position(center).unwrap_or_else(|e| {
            eprintln!("Failed to set cursor position: {:?}", e);
        });


    }



    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.player.camera.resize(new_size);
        self.renderer.resize(new_size);

        
    }



     pub fn update(&mut self,dt: std::time::Duration) {

        zone!("update state"); // <- Marca el inicio del bloque

        self.renderer.update();
        self.terrain.update(&self.renderer.queue, &self.player.camera.position);
        

        self.player.camera.update_dependants(dt);

        let cam_deps = &self.player.camera.dependants;

        self.renderer.update_consts(&mut self.data.globals, &[Globals::new(
            cam_deps.view_proj

        )])

    }

    pub fn handle_input_event(
        &mut self,
        event: &WindowEvent,
    ) -> bool {
        if self.state == GameState::PLAYING{
            self.player.camera.input_keyboard(&event)
        } else {
            false
        }
        
    }

    pub fn handle_device_input(&mut self, event: &DeviceEvent, _: &EventLoopWindowTarget<()>) {
        
        if self.state == GameState::PLAYING {
            self.player.camera.input(event);
        }
    }

    
}