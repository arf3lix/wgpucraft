use std::sync::{Arc, Mutex, RwLock};


use cgmath::Vector3;
use rayon::iter::{IntoParallelIterator, ParallelIterator};


use crate::render::{atlas::MaterialType, mesh::Mesh, pipelines::terrain::BlockVertex};


use super::{biomes::BiomeParameters, block::{Block, Direction}, noise::NoiseGenerator, world::LAND_LEVEL};


pub const CHUNK_Y_SIZE: usize = 100;
pub const CHUNK_AREA: usize = 16;
pub const CHUNK_AREA_WITH_PADDING: usize = CHUNK_AREA + 2; // +1 en cada lado
pub const TOTAL_CHUNK_SIZE: usize = CHUNK_Y_SIZE * CHUNK_AREA_WITH_PADDING * CHUNK_AREA_WITH_PADDING;


pub type Blocks = Vec<Vec<Vec<Block>>>;




fn init_blocks(offset: [i32; 3]) -> Blocks {
    let mut blocks = vec![
        vec![
            vec![
                Block::new(
                    MaterialType::DEBUG,
                    [0, 0, 0],
                    offset
                )
                ; CHUNK_AREA_WITH_PADDING
            ]; CHUNK_AREA_WITH_PADDING
        ]; CHUNK_Y_SIZE
    ];
   
    for y in 0..CHUNK_Y_SIZE {
        for z in 0..CHUNK_AREA_WITH_PADDING {
            for x in 0..CHUNK_AREA_WITH_PADDING {
                let position = Vector3 { 
                    x: x as i32 - 1,  // -1 para que el padding izquierdo sea x=-1
                    y: y as i32, 
                    z: z as i32 - 1   // -1 para que el padding frontal sea z=-1
                };
                
                let material_type = if y < 12 {
                    MaterialType::DEBUG
                } else if y == 12 {
                    MaterialType::DEBUG
                } else {
                    MaterialType::AIR
                };

                blocks[y][x][z] = Block::new(material_type, position.into(), offset);
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


    pub fn get_chunk_index_by_offset(&self, offset: &[i32; 3]) -> Option<usize> {
        self.offset_array.iter().position(|chunk_offset| {
            *chunk_offset.read().unwrap() == *offset
        })
    }


    pub fn pos_in_chunk_bounds(pos: Vector3<i32>) -> bool {
        // Ahora acepta posiciones desde -1 hasta CHUNK_AREA (0..15 es el área interna, -1 y 16 son padding)
        pos.x >= -1 && pos.y >= 0 && pos.z >= -1 &&
        pos.x <= CHUNK_AREA as i32 && 
        pos.y < CHUNK_Y_SIZE as i32 && 
        pos.z <= CHUNK_AREA as i32
    }


   
}


pub fn generate_chunk(blocks: &mut Blocks, offset: [i32; 3], seed: u32, biome: &BiomeParameters) {
    tracy_client::span!("generate chunk: full scope"); // Span por hilo

    let noise_generator = NoiseGenerator::new(seed);

    for y in 0..CHUNK_Y_SIZE {
        for x in 0..CHUNK_AREA_WITH_PADDING {
            for z in 0..CHUNK_AREA_WITH_PADDING {
                // Convertir coordenadas con padding a coordenadas de mundo
                let local_x = x as i32 - 1;
                let local_z = z as i32 - 1;
                let world_pos = local_pos_to_world(offset, Vector3::new(local_x, y as i32, local_z));
                let height_variation = noise_generator.get_height(world_pos.x as f32, world_pos.z as f32, biome.frequency, biome.amplitude);
                let new_height = (biome.base_height + height_variation).round() as usize;

                //let new_height = y;

                let block_type = if y > new_height {
                    if y <= LAND_LEVEL {
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

                blocks[y][x][z].update(block_type, offset);
            }
        }
    };
}


pub fn generate_blocks_independent(offset: [i32; 3], seed: u32, biome: &BiomeParameters) -> Blocks {
    let mut blocks = Blocks::new();
    generate_chunk(&mut blocks, offset, seed, biome);
    blocks
}