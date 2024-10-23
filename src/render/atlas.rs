use anyhow::*;

use crate::render::texture::*;
use crate::world::block::*;

use super::pipelines::GlobalsLayouts;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MaterialType {
    DIRT,
    GRASS,
    ROCK,
    WATER,
    AIR,
    DEBUG,
}

impl MaterialType {
    // Función asociada para crear un estado activo
    pub fn is_transparent(&self) -> bool {
        match self {
            MaterialType::AIR => true, // Devuelve true si es AIR
            _ => false, // Devuelve false para cualquier otro material
        }
    }
}




impl MaterialType {
    pub fn get_texture_coordinates(&self, texture_corner: [u32; 2], quad_side: Direction) -> [f32; 2] {
        match self {
            MaterialType::GRASS => match quad_side {
                Direction::TOP => atlas_pos_to_coordinates([0.0, 0.0], texture_corner),
                Direction::BOTTOM => atlas_pos_to_coordinates([2.0, 0.0], texture_corner),
                Direction::RIGHT => atlas_pos_to_coordinates([3.0, 0.0], texture_corner),
                Direction::LEFT => atlas_pos_to_coordinates([3.0, 0.0], texture_corner),
                Direction::FRONT => atlas_pos_to_coordinates([3.0, 0.0], texture_corner),
                Direction::BACK => atlas_pos_to_coordinates([3.0, 0.0], texture_corner),
            },
            MaterialType::DIRT => atlas_pos_to_coordinates([2.0, 0.0], texture_corner),
            MaterialType::ROCK => atlas_pos_to_coordinates([1.0, 0.0], texture_corner),
            MaterialType::WATER => atlas_pos_to_coordinates([13.0, 0.0], texture_corner),
            MaterialType::AIR => [0.0, 0.0],
            MaterialType::DEBUG => atlas_pos_to_coordinates([15.0, 3.0], texture_corner),
        }
    }
}

const BLOCK_PIXEL_SIZE: f32 = 16.0;
const ATLAS_PIXEL_SIZE: f32 = 256.0;

fn atlas_pos_to_coordinates(atlas_pos: [f32; 2], texture_corner: [u32; 2]) -> [f32; 2] {
    let mut pixel_x = atlas_pos[0] * BLOCK_PIXEL_SIZE;
    let mut pixel_y = atlas_pos[1] * BLOCK_PIXEL_SIZE;

    if texture_corner[0] == 1 {
        pixel_x += 15.0;
    }

    if texture_corner[1] == 1 {
        pixel_y += 16.0;
    }

    return [pixel_x / ATLAS_PIXEL_SIZE, pixel_y / ATLAS_PIXEL_SIZE];
}

pub struct Atlas {
    pub texture: Texture,
    pub bind_group: wgpu::BindGroup,

}

impl Atlas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, layouts: &GlobalsLayouts) -> Result<Self> {

        let diffuse_bytes = include_bytes!("../../assets/images/textures_atlas.png");
        let texture = Texture::from_bytes(&device, &queue, diffuse_bytes, "blocks.png").unwrap();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layouts.atlas_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        Ok(Self {
            texture,
            bind_group,
        })
    }
}
