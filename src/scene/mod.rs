use wgpu::BindGroup;
use winit::event::WindowEvent;

use crate::{render::{pipelines::{GlobalModel, Globals}, renderer::Renderer}, GameState};

use self::{camera::Camera, terrain::Terrain};

pub mod camera;
pub mod terrain;




pub struct Scene {
    pub data: GlobalModel,
    pub globals_bind_group: BindGroup,
    pub camera: Camera,
    pub terrain: Terrain,
}

impl Scene {
    /// Create a new `Scene` with default parameters.
    pub fn new(
        renderer: &mut Renderer
,
        // settings: &Settings,
    ) -> Self {

        let data = GlobalModel {
            globals: renderer.create_consts(&[Globals::default()]),

        };

        let globals_bind_group = renderer.bind_globals(&data);

        let camera = Camera::new(&renderer, (5.0, 12.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));

        let terrain = Terrain::new(
            &renderer,
        );

        



        Self {
            data,
            globals_bind_group,
            camera,
            terrain,

    
        }
    }

    pub fn update 
    (
        &mut self,
        renderer: &mut Renderer,
        dt: std::time::Duration

    ) {

        self.camera.update_dependants(dt);

        let cam_deps = &self.camera.dependants;

        renderer.update_consts(&mut self.data.globals, &[Globals::new(
            cam_deps.view_proj

        )])

    }

    pub fn handle_input_event(
        &mut self,
        event: &WindowEvent,
        game_state: &GameState
    ) -> bool {
        if *game_state == GameState::PLAYING{
            self.camera.input_keyboard(&event)
        } else {
            false
        }
        
    }
}