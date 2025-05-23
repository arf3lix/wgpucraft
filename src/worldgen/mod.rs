use crate::player::{camera::{Camera, Dependants}, Player};
use cgmath::point3;
use tracy::zone;
use wgpu::BindGroup;
use winit::event::WindowEvent;
use world::World;


use crate::{hud::HUD, render::{pipelines::{GlobalModel, Globals}, renderer::Renderer}, GameState};


pub mod world;
pub mod block;
pub mod chunk;
pub mod biomes;
pub mod noise;



pub struct Scene {
    pub data: GlobalModel,
    pub globals_bind_group: BindGroup,
    pub player: Player,
    pub terrain: World,
    pub last_player_pos: cgmath::Point3<f32>,
    pub hud: HUD,
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

        let hud = HUD::new(
            &renderer,
            &renderer.layouts.global,   
            renderer.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("HUD Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../../assets/shaders/hud.wgsl").into()),
            }),
        );

        let globals_bind_group = renderer.bind_globals(&data);

        let camera = Camera::new(&renderer, (8.0, 12.0, 8.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));

        let player = Player::new(camera);

        let terrain = World::new(
            &renderer,
        );

        



        Self {
            data,
            globals_bind_group,
            player,
            terrain,
            last_player_pos: point3(0.0, 0.0, 0.0),
            hud

    
        }
    }

    pub fn update 
    (
        &mut self,
        renderer: &mut Renderer,
        dt: std::time::Duration

    ) {

        zone!("update scene"); // <- Marca el inicio del bloque


        //println!("camera pos: {:?}", self.camera.position);


        //


        self.terrain.update(&renderer.queue, &self.player.camera.position);
        

        self.player.camera.update_dependants(dt);

        let cam_deps = &self.player.camera.dependants;

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
            self.player.camera.input_keyboard(&event)
        } else {
            false
        }
        
    }
}