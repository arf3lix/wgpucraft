use std::sync::{Arc, Mutex, RwLock};


use cgmath::Vector3;
use rayon::iter::{IntoParallelIterator, ParallelIterator};


use crate::render::{atlas::MaterialType, mesh::Mesh, pipelines::terrain::BlockVertex};


use super::block::{Block, Direction};


pub const CHUNK_Y_SIZE:usize = 100;
pub const CHUNK_AREA:usize =16;
pub const TOTAL_CHUNK_SIZE: usize = CHUNK_Y_SIZE * CHUNK_AREA * CHUNK_AREA;


pub type Blocks = Vec<Vec<Vec<Arc<Mutex<Block>>>>>;




fn init_blocks(offset: [i32; 3]) -> Blocks {


    let mut blocks = vec![
        vec![
            vec![
                Arc::new(
                    Mutex::new(
                        Block::new(
                            MaterialType::DEBUG,
                            [0, 0, 0],
                            offset
                        )
                    )
                )
                ; CHUNK_AREA
            ]; CHUNK_AREA
        ]; CHUNK_Y_SIZE
    ];
   
    // Assuming CHUNK_Y_SIZE is a usize or similar that represents the height.
    for y in 0..CHUNK_Y_SIZE{
        for z in 0..CHUNK_AREA {
            for x in 0..CHUNK_AREA {
                let position = cgmath::Vector3 { x: x as i32, y: y as i32, z: z as i32 };
                let material_type =
                if y < 12 {
                    MaterialType::DEBUG
                }
                else if y == 12{
                    MaterialType::DEBUG
                }
                else {
                    MaterialType::AIR
                };


                blocks[y][x][z] = Arc::new(Mutex::new(Block::new(material_type, position.into(), offset)));
            }
        }
    }


    blocks


}


pub fn local_pos_to_world(offset:[i32;3], local_pos: Vector3<i32>) -> Vector3<f32> {
    Vector3::new(
        local_pos.x as f32 + (offset[0] as f32 * CHUNK_AREA as f32),
        local_pos.y as f32 + (offset[1] as f32 * CHUNK_AREA as f32),
        local_pos.z as f32 + (offset[2] as f32 * CHUNK_AREA as f32)
    )
}


#[derive(Default)]
pub struct ChunkArray {
    pub mesh_array: Vec<Arc<RwLock<Mesh<BlockVertex>>>>,
    pub offset_array: Vec<Arc<RwLock<[i32; 3]>>>,
    pub blocks_array: Vec<Arc<RwLock<Blocks>>>,
}






impl ChunkArray {


    pub fn new_chunk(&mut self, offset: [i32; 3]) -> &Self {
        let blocks = init_blocks(offset);
        self.mesh_array.push(Arc::new(RwLock::new(Mesh::new())));
        self.blocks_array.push(Arc::new(RwLock::new(blocks)));
        self.offset_array.push(Arc::new(RwLock::new(offset)));
        return self;
    }

    fn get_block_at(&self, pos: Vector3<i32>, chunk_offset: &[i32; 3]) -> Option<Arc<Mutex<Block>>> {
        let (x, y, z) = (pos.x, pos.y, pos.z);

        if y < 0 || y >= CHUNK_Y_SIZE as i32 {
            // Outside vertical bounds
            return None;
        }

        let mut local_x = x;
        let mut local_z = z;
        let mut local_chunk_offset = chunk_offset.clone();

        // Adjust for x-axis
        if x < 0 {
            local_x += CHUNK_AREA as i32;
            local_chunk_offset[0] -= 1;
        } else if x >= CHUNK_AREA as i32 {
            local_x -= CHUNK_AREA as i32;
            local_chunk_offset[0] += 1;
        }

        // Adjust for z-axis
        if z < 0 {
            local_z += CHUNK_AREA as i32;
            local_chunk_offset[2] -= 1;
        } else if z >= CHUNK_AREA as i32 {
            local_z -= CHUNK_AREA as i32;
            local_chunk_offset[2] += 1;
        }

        // Get chunk index by offset
        let chunk_index = self.get_chunk_index_by_offset(&local_chunk_offset)?;

        let blocks = self.blocks_array[chunk_index].read().unwrap();
        Some(blocks[y as usize][local_x as usize][local_z as usize].clone())
    }

    fn get_chunk_index_by_offset(&self, offset: &[i32; 3]) -> Option<usize> {
        self.offset_array.iter().position(|chunk_offset| {
            *chunk_offset.read().unwrap() == *offset
        })
    }

    fn should_render_face(&self, blocks: &Blocks, pos: (i32, i32, i32), direction: Direction, chunk_offset: &[i32; 3]) -> bool {
        let dir_vec = direction.to_vec();
        let neighbor_pos = Vector3::new(pos.0, pos.1, pos.2) + dir_vec;

        if neighbor_pos.y < 0 || neighbor_pos.y >= CHUNK_Y_SIZE as i32 {
            // Faces at the top and bottom are visible
            return true;
        }

        if neighbor_pos.x >= 0 && neighbor_pos.x < CHUNK_AREA as i32 && neighbor_pos.z >= 0 && neighbor_pos.z < CHUNK_AREA as i32 {
            // Neighbor is within the same chunk
            let neighbor_block = blocks[neighbor_pos.y as usize][neighbor_pos.x as usize][neighbor_pos.z as usize].lock().unwrap();
            return neighbor_block.is_transparent();
        } else {
            // Neighbor is in a different chunk
            if let Some(neighbor_block) = self.get_block_at(neighbor_pos, chunk_offset) {
                let neighbor_block = neighbor_block.lock().unwrap();
                return neighbor_block.is_transparent();
            } else {
                // Neighboring chunk is not loaded; assume the face is visible
                return true;
            }
        }
    }

    pub fn generate_chunk_mesh(&self, chunk_index: usize) {
        let blocks = self.blocks_array[chunk_index].read().unwrap();
        let offset = self.offset_array[chunk_index].read().unwrap();
        let mut mesh = Mesh::new();

        let mut vertex_offset = 0u16;

        for y in 0..CHUNK_Y_SIZE {
            for z in 0..CHUNK_AREA {
                for x in 0..CHUNK_AREA {
                    let block = blocks[y][x][z].lock().unwrap();

                    if !block.is_solid() {
                        continue; // Skip non-solid blocks
                    }

                    let block_pos = (x as i32, y as i32, z as i32);

                    for &direction in &[
                        Direction::TOP,
                        Direction::BOTTOM,
                        Direction::RIGHT,
                        Direction::LEFT,
                        Direction::FRONT,
                        Direction::BACK,
                    ] {
                        if self.should_render_face(&blocks, block_pos, direction, &offset) {
                            let quad = &block.quads[direction as usize];

                            mesh.verts.extend_from_slice(&quad.vertices);

                            let indices = quad.get_indices(vertex_offset);
                            mesh.indices.extend_from_slice(&indices);

                            vertex_offset += 4; // Each quad has 4 vertices
                        }
                    }
                }
            }
        }

        // Update the mesh in the mesh_array
        *self.mesh_array[chunk_index].write().unwrap() = mesh;
    }








    pub fn pos_in_chunk_bounds(pos: Vector3<i32>) -> bool {
        if pos.x >= 0 && pos.y >= 0 && pos.z >= 0 {
            if pos.x < CHUNK_AREA as i32 && pos.y < CHUNK_Y_SIZE as i32 && pos.z < CHUNK_AREA as i32 {
                return true;
            }
        }
        return false;
    }



   
}


pub fn generate_chunk(blocks: &mut Blocks, offset: [i32; 3]) {
    (0..TOTAL_CHUNK_SIZE).into_par_iter().for_each(|i| {
        let z = i / (CHUNK_AREA * CHUNK_Y_SIZE);
        let y = (i - z * CHUNK_AREA * CHUNK_Y_SIZE) / CHUNK_AREA;
        let x = i % CHUNK_AREA;


        // Función matemática simple para generar un terreno 3D con colinas suaves
        let base_height = 10.0;
        let frequency = 0.1;
        let amplitude = 5.0;
        
        let height_variation = (x as f32 * frequency).sin() + (z as f32 * frequency).sin();
        let new_height = (base_height + height_variation * amplitude).round() as usize;

        let block_type = if y > new_height {
            if y <=     12 {
                MaterialType::WATER
            } else {
                MaterialType::AIR
            }
        } else if y == new_height {
            MaterialType::GRASS
        } else if y == 0 {
            MaterialType::ROCK
        } else {
            MaterialType::DIRT
        };


        blocks[y][x][z].lock().unwrap().update(block_type, offset);
    });
}

