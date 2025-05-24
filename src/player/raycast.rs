
use cgmath::{InnerSpace, Vector3};
use crate::{render::atlas::MaterialType, terrain_gen::{block::Direction, chunk::ChunkManager}};
use super::camera::Camera;


pub struct Ray {
    pub origin: cgmath::Point3<f32>,
    pub direction: Vector3<f32>,
    pub length: f32,
}

pub struct BlockHit {
    pub position: Vector3<i32>,
    pub face: Direction,
    pub distance: f32,
}


impl BlockHit {
    pub fn neighbor_position(&self) -> Vector3<i32> {
        self.position + self.face.to_vec()
    }
}


impl Ray {
    pub fn new(origin: cgmath::Point3<f32>, direction: Vector3<f32>, length: f32) -> Self {
        Self {
            origin,
            direction,
            length,
        }
    }


    pub fn from_camera(camera: &Camera, length: f32) -> Self {
        let (sin_pitch, cos_pitch) = camera.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = camera.yaw.0.sin_cos();
        
        let direction = Vector3::new(
            cos_pitch * cos_yaw,
            sin_pitch,
            cos_pitch * sin_yaw
        ).normalize();

        Self {
            origin: camera.position,
            direction,
            length,
        }
    }


    pub fn cast(&self, chunk_array: &ChunkManager) -> Option<BlockHit> {
        // Convertimos el origen a coordenadas de bloque
        let mut current_block_pos = Vector3::new(
            self.origin.x.floor() as i32,
            self.origin.y.floor() as i32,
            self.origin.z.floor() as i32,
        );

        // Calculamos el paso y la distancia inicial para cada eje
        let step = Vector3::new(
            if self.direction.x > 0.0 { 1 } else { -1 },
            if self.direction.y > 0.0 { 1 } else { -1 },
            if self.direction.z > 0.0 { 1 } else { -1 },
        );

        let next_boundary = Vector3::new(
            if step.x > 0 {
                current_block_pos.x as f32 + 1.0
            } else {
                current_block_pos.x as f32
            },
            if step.y > 0 {
                current_block_pos.y as f32 + 1.0
            } else {
                current_block_pos.y as f32
            },
            if step.z > 0 {
                current_block_pos.z as f32 + 1.0
            } else {
                current_block_pos.z as f32
            },
        );

        let mut t_max = Vector3::new(
            if self.direction.x != 0.0 {
                (next_boundary.x - self.origin.x) / self.direction.x
            } else {
                f32::INFINITY
            },
            if self.direction.y != 0.0 {
                (next_boundary.y - self.origin.y) / self.direction.y
            } else {
                f32::INFINITY
            },
            if self.direction.z != 0.0 {
                (next_boundary.z - self.origin.z) / self.direction.z
            } else {
                f32::INFINITY
            },
        );

        let t_delta = Vector3::new(
            if self.direction.x != 0.0 {
                step.x as f32 / self.direction.x
            } else {
                f32::INFINITY
            },
            if self.direction.y != 0.0 {
                step.y as f32 / self.direction.y
            } else {
                f32::INFINITY
            },
            if self.direction.z != 0.0 {
                step.z as f32 / self.direction.z
            } else {
                f32::INFINITY
            },
        );

        let mut face = Direction::TOP; // Valor por defecto, se actualizará
        let mut traveled_distance = 0.0;

        // Algoritmo DDA
        while traveled_distance < self.length {
            // Verificamos si el bloque actual es sólido
            if let Some(material) = chunk_array.get_block(current_block_pos) {
                if material != MaterialType::AIR && material != MaterialType::WATER {
                    return Some(BlockHit {
                        position: current_block_pos,
                        face,
                        distance: traveled_distance,
                    });
                }
            }

            // Avanzamos al siguiente bloque
            if t_max.x < t_max.y && t_max.x < t_max.z {
                traveled_distance = t_max.x;
                t_max.x += t_delta.x;
                current_block_pos.x += step.x;
                face = if step.x > 0 {
                    Direction::LEFT
                } else {
                    Direction::RIGHT
                };
            } else if t_max.y < t_max.z {
                traveled_distance = t_max.y;
                t_max.y += t_delta.y;
                current_block_pos.y += step.y;
                face = if step.y > 0 {
                    Direction::BOTTOM
                } else {
                    Direction::TOP
                };
            } else {
                traveled_distance = t_max.z;
                t_max.z += t_delta.z;
                current_block_pos.z += step.z;
                face = if step.z > 0 {
                    Direction::BACK
                } else {
                    Direction::FRONT
                };
            }
        }

        None
    }


}