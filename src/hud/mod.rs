use crate::render::{mesh::Mesh, model::Model, pipelines::{hud::{create_hud_pipeline, HUDVertex}, GlobalsLayouts}, renderer::{self, Draw, Renderer}, texture::Texture};








pub struct HUD {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub crosshair: HUDElement,
    pub widget: HUDElement,
}

struct HUDElement {
    texture: Texture,
    bind_group: wgpu::BindGroup,
    model: Model<HUDVertex>,
}


impl HUD {
    pub fn new(
        renderer: &Renderer,
        global_layout: &GlobalsLayouts,
        shader: wgpu::ShaderModule,
    ) -> Self {
        // Cargar texturas
        let crosshair_bytes = include_bytes!("../../assets/images/crosshair.png");
        let widget_bytes = include_bytes!("../../assets/images/widget_window.png");

        let crosshair_tex = Texture::from_bytes(&renderer.device, &renderer.queue, crosshair_bytes, "crosshair.png").unwrap();
        let widget_tex = Texture::from_bytes(&renderer.device, &renderer.queue, widget_bytes, "crosshair.png").unwrap();

        // Crear pipeline usando el hud_layout
        let pipeline = create_hud_pipeline(
            &renderer.device, 
            &global_layout, // Usamos el layout específico
            shader, 
            &renderer.config
        );

        // Crear bind groups usando el nuevo método
        let crosshair_bind_group = global_layout.bind_hud_texture(
            &renderer.device,
            &crosshair_tex,
            None, // Usa el sampler por defecto
        );

        let widget_bind_group = global_layout.bind_hud_texture(
            &renderer.device,
            &widget_tex,
            None,
        );

        // Crear geometría para los elementos del HUD
        let (crosshair_verts, crosshair_indices) = create_hud_quad(0.0, 0.0, 0.06, 0.06); // Ajusta tamaño según necesites
        let (widget_verts, widget_indices) = create_hud_quad(0.8, -0.8, 0.4, 0.4); // Posición y tamaño del widget



        
        // Crear modelos
        let crosshair_mesh = Mesh {
            verts: crosshair_verts,
            indices: crosshair_indices,
        };
        let widget_mesh = Mesh {
            verts: widget_verts,
            indices: widget_indices,
        };

        let crosshair_model = Model::new(&renderer.device, &crosshair_mesh).unwrap();
        let widget_model = Model::new(&renderer.device, &widget_mesh).unwrap();

        // Crear bind groups
        let crosshair = HUDElement {
            texture: crosshair_tex,
            bind_group: crosshair_bind_group,
            model: crosshair_model,
        };

        let widget = HUDElement {
            texture: widget_tex,
            bind_group: widget_bind_group,
            model: widget_model,
        };

        Self {
            pipeline,
            crosshair,
            widget,
        }
    }

    pub fn update(&mut self) {

    }
}

impl Draw for HUD {
    fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, globals: &'a wgpu::BindGroup) -> Result<(), wgpu::Error> {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(1, globals, &[]);
        
        // Dibujar elementos del HUD
        for element in &[&self.crosshair, &self.widget] {
            render_pass.set_bind_group(0, &element.bind_group, &[]);
            render_pass.set_vertex_buffer(0, element.model.vbuf().slice(..));
            render_pass.set_index_buffer(
                element.model.ibuf().slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.draw_indexed(0..element.model.num_indices as u32, 0, 0..1);
        }
        
        Ok(())
    } 
}


pub fn create_hud_quad(
    center_x: f32, 
    center_y: f32,
    width: f32,
    height: f32,
) -> (Vec<HUDVertex>, Vec<u16>) {
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    
    let vertices = vec![
        // Top Left
        HUDVertex {
            position: [center_x - half_w, center_y - half_h],
            uv: [0.0, 0.0],
        },
        // Top Right
        HUDVertex {
            position: [center_x + half_w, center_y - half_h],
            uv: [1.0, 0.0],
        },
        // Bottom Right
        HUDVertex {
            position: [center_x + half_w, center_y + half_h],
            uv: [1.0, 1.0],
        },
        // Bottom Left
        HUDVertex {
            position: [center_x - half_w, center_y + half_h],
            uv: [0.0, 1.0],
        },
    ];
    
    let indices = vec![0, 1, 2, 0, 2, 3];
    
    (vertices, indices)
}