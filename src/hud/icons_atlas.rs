use anyhow::*;

use crate::render::texture::*;
use super::HUDVertex;


#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IconType {
    ROCK,
    GRASS,
    DIRT,
    STONE,
    WOODEN,
}

const ICON_SIZE: (f32, f32) = (32.0, 32.0);
const TEXTURE_SIZE: (f32, f32) = (512.0, 512.0);

impl IconType {
    fn get_uv_cords(&self) -> [f32; 4] {
        let (x, y) = match self {
            IconType::ROCK => (0, 0),
            IconType::GRASS => (1, 0),
            IconType::DIRT => (2, 0),
            IconType::STONE => (3, 0),
            IconType::WOODEN => (4, 0),
            // Mapea más iconos según tu atlas
        };
        
        let u_min = (x as f32 * ICON_SIZE.0) / TEXTURE_SIZE.0;
        let v_min = (y as f32 * ICON_SIZE.1) / TEXTURE_SIZE.1;
        let u_max = u_min + (ICON_SIZE.0 / TEXTURE_SIZE.0);
        let v_max = v_min + (ICON_SIZE.1 / TEXTURE_SIZE.1);
        
        [u_min, v_min, u_max, v_max]
    }

    pub fn next(self) -> Self {
        match self {
            IconType::ROCK => IconType::GRASS,
            IconType::GRASS => IconType::DIRT,
            IconType::DIRT => IconType::STONE,
            IconType::STONE => IconType::WOODEN,
            IconType::WOODEN => IconType::ROCK, // circular
        }
    }

    pub fn prev(self) -> Self {
        match self {
            IconType::ROCK => IconType::WOODEN,
            IconType::WOODEN => IconType::STONE,
            IconType::STONE => IconType::DIRT,
            IconType::DIRT => IconType::GRASS,
            IconType::GRASS => IconType::ROCK,
        }
    }


    pub fn get_vertex_quad(
        &self,
        center_x: f32, 
        center_y: f32,
        width: f32,
        height: f32,
    ) -> ([HUDVertex; 4], [u16; 6]) {
        let uv = self.get_uv_cords();
        let half_width = width / 2.0;
        let half_height = height / 2.0;
        
        let vertices = [
            HUDVertex {
                position: [center_x - half_width, center_y - half_height],
                uv: [uv[0], uv[3]], // Nota v_min y v_max invertidos
            },
            HUDVertex {
                position: [center_x + half_width, center_y - half_height],
                uv: [uv[2], uv[3]],
            },
            HUDVertex {
                position: [center_x + half_width, center_y + half_height],
                uv: [uv[2], uv[1]],
            },
            HUDVertex {
                position: [center_x - half_width, center_y + half_height],
                uv: [uv[0], uv[1]],
            },
        ];

        // Índices para renderizar dos triángulos (un quad)
        let indices = [0, 1, 2, 2, 3, 0];

        (vertices, indices)
    }

}


