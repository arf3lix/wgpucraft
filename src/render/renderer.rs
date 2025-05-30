use std::time::Duration;

use tracy_client::span;
use wgpu::{BindGroup, CommandEncoder, Error};
use instant::Instant;
use winit::window::Window as SysWindow;


use crate::{hud::HUD, terrain_gen::generator::TerrainGen};
use super::{consts::Consts, pipelines::{GlobalModel, GlobalsLayouts}, texture::{self, Texture}};
pub trait Draw {
    fn draw<'a>(
        &'a self, 
        render_pass: &mut wgpu::RenderPass<'a>, 
        globals: &'a wgpu::BindGroup
    ) -> Result<(), Error>;
}


pub struct Layouts {
    pub global: GlobalsLayouts
}


pub struct Renderer<'a> {
    surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'a SysWindow,
    pub config: wgpu::SurfaceConfiguration,
    pub queue: wgpu::Queue,
    pub layouts: Layouts,
    depth_texture: Texture,
}

impl<'a> Renderer<'a> {
    pub fn new(
        window: &'a SysWindow,
    ) -> Self {
        let size = window.inner_size();
    
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        let surface = instance.create_surface(window).unwrap();
    
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        )).unwrap();

        let (device, queue) = pollster::block_on(adapter 
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::POLYGON_MODE_LINE|
                                        wgpu::Features::TIMESTAMP_QUERY |
                                        wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS|
                                        wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                },
                None,
            ) 
        ) 
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let layouts = Layouts { global: GlobalsLayouts::new(&device)};

        let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");




        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            layouts,
            depth_texture,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn bind_globals(
        &self,
        global_model: &GlobalModel,
    ) -> BindGroup {
        self.layouts
            .global
            .bind(&self.device, global_model)
    }

    pub fn update(&mut self) {
        //todo!();
    }

    pub fn create_consts<T: Copy + bytemuck::Pod>(
        &mut self,
        vals: &[T],
    ) -> Consts<T> {
        let mut consts = Consts::new(&self.device, vals.len());
        consts.update(&self.queue, vals, 0);
        consts
    }

    /// Update a set of constants with the provided values.
    pub fn update_consts<T: Copy + bytemuck::Pod>(&self, consts: &mut Consts<T>, vals: &[T]) {
        let _span = span!("update render constants"); // <- Marca el inicio del bloque

        consts.update(&self.queue, vals, 0)
    }

    pub fn render(&mut self, terrain: &TerrainGen, hud: &HUD, globals: &BindGroup) -> Result<(), wgpu::SurfaceError> {



        let get_texture_span = span!("get current texture");
        let output = self.surface.get_current_texture()?;
        drop(get_texture_span);
        

        let create_view_span = span!("create view");
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        drop(create_view_span);

        let create_encoder_span = span!("create encoder");
        let mut encoder = self.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                }
            );
        drop(create_encoder_span);

        // Crear y liberar explícitamente el render pass
        {
            let create_render_pass =  span!("create render pass");
            let mut _render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &self.depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }
            );
            drop(create_render_pass);

            terrain.draw(&mut _render_pass, globals).unwrap();
            
            hud.draw(&mut _render_pass, globals).unwrap();

            
        } // _render_pass se libera aquí
        
        
        // submit will accept anything that implements IntoIter
        let submit_encoder = span!("submit encoder");
        self.queue.submit(std::iter::once(encoder.finish()));
        drop(submit_encoder);
        let prenset_output = span!("present output");
        output.present();
        drop(prenset_output);
    
        Ok(())
    }

}


