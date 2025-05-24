use std::sync::{Arc, RwLock};


use cgmath::Vector3;
use tracy::zone;


use crate::render::{atlas::MaterialType, mesh::Mesh, pipelines::terrain::BlockVertex};


use super::{biomes::BiomeParameters, block::Block, noise::NoiseGenerator, generator::LAND_LEVEL};


pub const CHUNK_Y_SIZE: usize = 100;
pub const CHUNK_AREA: usize = 16;
pub const CHUNK_AREA_WITH_PADDING: usize = CHUNK_AREA + 2; // +1 en cada lado
pub const TOTAL_CHUNK_SIZE: usize = CHUNK_Y_SIZE * CHUNK_AREA_WITH_PADDING * CHUNK_AREA_WITH_PADDING;


pub struct Chunk {
    pub blocks: Vec<Block>,
    pub offset: [i32; 3],
    pub mesh: Mesh<BlockVertex>,
}

impl Chunk {
    pub fn new(offset: [i32; 3]) -> Self {
    let mut blocks = Vec::with_capacity(TOTAL_CHUNK_SIZE);

    for y in 0..CHUNK_Y_SIZE {
        for x in 0..CHUNK_AREA_WITH_PADDING {
            for z in 0..CHUNK_AREA_WITH_PADDING {
                let position = Vector3 {
                    x: x as i32 - 1,  // -1 para el padding izquierdo
                    y: y as i32,
                    z: z as i32 - 1,  // -1 para el padding frontal
                };

                let material_type = if y < 12 {
                    MaterialType::DEBUG
                } else if y == 12 {
                    MaterialType::DEBUG
                } else {
                    MaterialType::AIR
                };

                blocks.push(Block::new(material_type, position.into(), offset));
            }
        }
    }
        let mesh = Mesh::new();
        Chunk { blocks, offset, mesh }
    }


    /// Calcula el índice lineal basado en coordenadas y, x, z
    fn calculate_index(&self, y: usize, x: usize, z: usize) -> usize {
        y * (CHUNK_AREA_WITH_PADDING * CHUNK_AREA_WITH_PADDING) + 
        x * CHUNK_AREA_WITH_PADDING + 
        z
    }


    /// Obtiene una referencia inmutable a un bloque
    pub fn get_block(&self, y: usize, x: usize, z: usize) -> Option<&Block> {
        if y < CHUNK_Y_SIZE && x < CHUNK_AREA_WITH_PADDING && z < CHUNK_AREA_WITH_PADDING {
            let index = self.calculate_index(y, x, z);
            self.blocks.get(index)
        } else {
            None
        }
    }

    /// Obtiene una referencia mutable a un bloque
    pub fn get_block_mut(&mut self, y: usize, x: usize, z: usize) -> Option<&mut Block> {
        if y < CHUNK_Y_SIZE && x < CHUNK_AREA_WITH_PADDING && z < CHUNK_AREA_WITH_PADDING {
            let index = self.calculate_index(y, x, z);
            self.blocks.get_mut(index)
        } else {
            None
        }
    }



}





pub struct ChunkManager {
    pub chunks: Vec<Arc<RwLock<Chunk>>>,
}

impl ChunkManager {
    pub fn new() -> Self {
        ChunkManager {
            chunks: Vec::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: Chunk) {
        self.chunks.push(Arc::new(RwLock::new(chunk)));
    }

    pub fn get_chunk(&self, index: usize) -> Option<Arc<RwLock<Chunk>>> {
        if index < self.chunks.len() {
            Some(self.chunks[index].clone())
        } else {
            None
        }
    }

    pub fn get_chunk_index_by_offset(&self, offset: &[i32; 3]) -> Option<usize> {
        self.chunks.iter().position(|chunk| {
            chunk.read().unwrap().offset == *offset
        })
    }


    
    // Obtiene el material de un bloque en una posición mundial
    pub fn get_block(&self, world_pos: Vector3<i32>) -> Option<MaterialType> {
        let (chunk_offset, local_pos) = world_pos_to_chunk_and_local(world_pos);
        
        // Ajustamos para el padding (local_pos es 0..15, necesitamos -1..16)
        let x = local_pos.x + 1;
        let z = local_pos.z + 1;
        let y = local_pos.y;
        
        if !pos_in_chunk_bounds(Vector3::new(x, y, z)) {
            return None;
        }
        
        self.get_chunk_index_by_offset(&chunk_offset)
            .and_then(|index| {
                let chunk = self.chunks[index].read().unwrap();
                Some(chunk.get_block(y as usize, x as usize, z as usize)?.material_type)
            })
    }

    // Establece el material de un bloque en una posición mundial
    pub fn set_block(&mut self, world_pos: Vector3<i32>, material: MaterialType) -> Option<usize> {
        let (chunk_offset, local_pos) = world_pos_to_chunk_and_local(world_pos);
        
        // Ajustamos para el padding (local_pos es 0..15, necesitamos -1..16)
        let x = local_pos.x + 1;
        let z = local_pos.z + 1;
        let y = local_pos.y;
        
        if !pos_in_chunk_bounds(Vector3::new(x, y, z)) {
            println!("Position out of bounds: {:?}", world_pos);
            return None;
        }
        
        if let Some(index) = self.get_chunk_index_by_offset(&chunk_offset) {
            let mut chunk = self.chunks[index].write().unwrap();
            let block = chunk.get_block_mut(y as usize, x as usize, z as usize)?;
            block.update(material, chunk_offset);
            println!("Block updated at world position: {:?}", world_pos);
            return Some(index);
        } else {
            println!("Chunk not found for world position: {:?}", world_pos);
            return None;
        }
    }

    
}




pub fn generate_chunk(chunk: &mut Chunk, offset: [i32; 3], noise_generator: &NoiseGenerator, biome: &BiomeParameters) {
    zone!("generate chunk: full scope"); // Span por hilo

    let max_biome_height = (biome.base_height + biome.amplitude) as usize;


    for y in 0..CHUNK_Y_SIZE {
        for x in 0..CHUNK_AREA_WITH_PADDING {
            for z in 0..CHUNK_AREA_WITH_PADDING {

                if y > max_biome_height {
                    continue; // Saltamos al siguiente bloque
                }

                zone!(" creating single block"); // Span por hilo


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

                chunk.get_block_mut(y, x, z).unwrap().update(block_type, offset);
            }
        }
    };
}



pub fn pos_in_chunk_bounds(pos: Vector3<i32>) -> bool {
    // Ahora acepta posiciones desde -1 hasta CHUNK_AREA (0..15 es el área interna, -1 y 16 son padding)
    pos.x >= -1 && pos.y >= 0 && pos.z >= -1 &&
    pos.x <= CHUNK_AREA as i32 && 
    pos.y < CHUNK_Y_SIZE as i32 && 
    pos.z <= CHUNK_AREA as i32
}


fn world_pos_to_chunk_and_local(world_pos: Vector3<i32>) -> ([i32; 3], Vector3<i32>) {
    let chunk_x = world_pos.x.div_euclid(CHUNK_AREA as i32);
    let chunk_y = world_pos.y.div_euclid(CHUNK_Y_SIZE as i32);
    let chunk_z = world_pos.z.div_euclid(CHUNK_AREA as i32);
    
    let local_x = world_pos.x.rem_euclid(CHUNK_AREA as i32);
    let local_y = world_pos.y.rem_euclid(CHUNK_Y_SIZE as i32);
    let local_z = world_pos.z.rem_euclid(CHUNK_AREA as i32);
    
    ([chunk_x, chunk_y, chunk_z], Vector3::new(local_x, local_y, local_z))
}

pub fn local_pos_to_world(offset:[i32;3], local_pos: Vector3<i32>) -> Vector3<f32> {
    Vector3::new(
        local_pos.x as f32 + (offset[0] as f32 * CHUNK_AREA as f32),
        local_pos.y as f32 + (offset[1] as f32 * CHUNK_AREA as f32),
        local_pos.z as f32 + (offset[2] as f32 * CHUNK_AREA as f32)
    )
}