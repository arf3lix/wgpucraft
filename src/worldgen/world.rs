
use std::{collections::VecDeque, sync::{Arc, RwLock, Barrier}};
use rayon::ThreadPoolBuilder;

use crate::{render::{atlas::{Atlas, MaterialType}, mesh::Mesh, model::DynamicModel, pipelines::terrain::{BlockVertex, create_terrain_pipeline}, renderer::{Draw, Renderer}}, worldgen::biomes::PRAIRIE_PARAMS};
use crate::render::pipelines::GlobalsLayouts;
use crate::worldgen::chunk::{self, generate_chunk, Blocks, ChunkArray, CHUNK_AREA, CHUNK_Y_SIZE};
use tracy::zone;


use cgmath::{EuclideanSpace, Point3, Vector3};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use wgpu::Queue;


pub const LAND_LEVEL: usize = 9;
pub const CHUNKS_VIEW_SIZE: usize = 16;
pub const CHUNKS_ARRAY_SIZE: usize = CHUNKS_VIEW_SIZE * CHUNKS_VIEW_SIZE;


use super::{biomes::BiomeParameters, noise::NoiseGenerator};






pub struct World {
    pipeline: wgpu::RenderPipeline,
    atlas: Atlas,
    pub chunks: ChunkArray,
    chunk_indices: Arc<RwLock<[Option<usize>; CHUNKS_ARRAY_SIZE]>>,
    free_chunk_indices: Arc<RwLock<VecDeque<usize>>>,
    updated_indices: Arc<RwLock<[bool; CHUNKS_ARRAY_SIZE]>>,
    center_offset: Vector3<i32>,
    chunks_origin: Vector3<i32>,
    pub chunk_models: Vec<DynamicModel<BlockVertex>>,
    noise_gen: NoiseGenerator


}


impl World {
    pub fn new(renderer: &Renderer) -> Self {
        let global_layouts = GlobalsLayouts::new(&renderer.device);
        let atlas = Atlas::new(&renderer.device, &renderer.queue, &global_layouts).unwrap();
        let mut chunk_models = vec![];
        let mut chunks:ChunkArray = ChunkArray::default();
        let chunk_indices: [Option<usize>; CHUNKS_ARRAY_SIZE] = [None; CHUNKS_ARRAY_SIZE];
        let updated_indices = Arc::new(RwLock::new([false; CHUNKS_ARRAY_SIZE]));
        let mut free_chunk_indices = VecDeque::new();

        let noise_gen = NoiseGenerator::new(10);



        for x in 0..CHUNKS_ARRAY_SIZE {
            //println!("initial x from new World: {:?}", x);
            chunks.new_chunk([0,0,0]);
            let mut chunk_model = DynamicModel::new(&renderer.device, (CHUNK_AREA ^ 2) * CHUNK_Y_SIZE * 24);
            chunk_model.update(&renderer.queue, &chunks.mesh_array[x].read().unwrap(), 0);
            chunk_models.push(chunk_model);
            free_chunk_indices.push_back(x);


        }


        let shader = renderer.device.create_shader_module(
            wgpu::include_wgsl!("../../assets/shaders/shader.wgsl")
        );


        let world_pipeline = create_terrain_pipeline(
            &renderer.device,
            &global_layouts,
            shader,
            &renderer.config
        );


        let center_offset = Vector3::new(0, 0, 0);
        let chunks_origin = center_offset - Vector3::new(CHUNKS_VIEW_SIZE as i32 / 2, 0, CHUNKS_VIEW_SIZE as i32 / 2);


        let mut world = Self {
            pipeline: world_pipeline,
            atlas,
            chunks,
            chunk_models,
            center_offset,
            chunks_origin,
            updated_indices,
            chunk_indices: Arc::new(RwLock::new(chunk_indices)),
            free_chunk_indices: Arc::new(RwLock::new(free_chunk_indices)),
            noise_gen
        };


        println!("about to load first chunks");
        world.load_initial_chunks(&renderer.queue);


        world
    }






    pub fn load_initial_chunks(&mut self, queue: &Queue) {

        zone!("load initial chunks"); // <- Marca el inicio del bloque

        let chunks_to_update: Vec<usize> = (0..CHUNKS_ARRAY_SIZE)
            .filter(|&i| self.chunk_indices.read().unwrap()[i].is_none())
            .collect();

        // Primer bucle: Generar los chunks
        for i in chunks_to_update.iter() {
            let new_index = self.free_chunk_indices.write().unwrap().pop_front();
            if let Some(new_index) = new_index {
                let chunk_offset = self.get_chunk_offset(*i);
                if !self.chunk_in_bounds(chunk_offset) {
                    panic!("Error: Cannot load chunk");
                }
                
                *self.chunks.offset_array[new_index].write().unwrap() = chunk_offset.into();
                
                generate_chunk(
                    &mut self.chunks.blocks_array[new_index].write().unwrap(),
                    chunk_offset.into(),
                    &self.noise_gen,
                    &PRAIRIE_PARAMS
                );

                self.chunk_indices.write().unwrap()[*i] = Some(new_index);
            } else {
                panic!("Error: No free space for chunk");
            }
        }

        // Segundo bucle: Calcular los meshes
        for i in chunks_to_update.iter() {
            if let Some(new_index) = self.chunk_indices.read().unwrap()[*i] {
                let mesh = self.update_mesh(
                    &self.chunks.blocks_array[new_index].read().unwrap(),
                    PRAIRIE_PARAMS
                );
                *self.chunks.mesh_array[new_index].write().unwrap() = mesh;
            }
        }

        // Actualizar los modelos de chunk
        (0..CHUNKS_ARRAY_SIZE).for_each(|i| {
            zone!("load chunk model"); // <- Marca el inicio del bloque

            //self.chunk_models[i].update(queue, &self.chunks[i].read().unwrap().mesh, 0);
            self.chunk_models[i].update(queue, &self.chunks.mesh_array[i].read().unwrap(), 0);
        });

        println!("---------------------------------");
    }





    pub fn load_empty_chunks(&mut self, queue: &Queue) {
        zone!("load empty chunks"); // <- Marca el inicio del bloque

        let chunks_to_update: Vec<usize> = (0..CHUNKS_ARRAY_SIZE)
            .filter(|&i| self.chunk_indices.read().unwrap()[i].is_none())
            .collect();

        chunks_to_update.into_par_iter().for_each(|i| {
            zone!(" ldc: thread_work"); // Span por hilo

            //let c = Arc::clone(&barrier); // Clonar la referencia a la barrera para cada hilo

            //println!("thread_id {:?}", thread_id);

            let new_index = self.free_chunk_indices.write().unwrap().pop_front();
            if let Some(new_index) = new_index {
                let chunk_offset = self.get_chunk_offset(i);
                if !self.chunk_in_bounds(chunk_offset) {
                    panic!("Error: Cannot load chunk")
                }
                
                *self.chunks.offset_array[new_index].write().unwrap() = chunk_offset.into();
                
                generate_chunk(
                    &mut self.chunks.blocks_array[new_index].write().unwrap(),
                    chunk_offset.into(),
                    &self.noise_gen,
                    &PRAIRIE_PARAMS
                );
                //println!("just gen chunk at thread_id {:?}", thread_id);

                self.chunk_indices.write().unwrap()[i] = Some(new_index);

                // Esperar hasta que todos los hilos hayan llegado a este punto
                //c .wait();
                //println!("about to calculate mesh at thread_id {:?}", thread_id);
                let mesh = self.update_mesh(
                    &self.chunks.blocks_array[new_index].read().unwrap(),
                    PRAIRIE_PARAMS
                );
                *self.chunks.mesh_array[new_index].write().unwrap() = mesh;

                // Marcar este índice como actualizado
                //self.updated_indices.write().unwrap()[new_index] = true;

            } else {
                panic!("Error: No free space for chunk")
            }
        
        });


        (0..CHUNKS_ARRAY_SIZE).for_each(|i| {
            //println!("trying to update");

            self.chunk_models[i].update(queue, &self.chunks.mesh_array[i].read().unwrap(), 0);

            
        });


        //println!("---------------------------------");
    }



    pub fn update_mesh(&self, blocks: &Blocks, biome: BiomeParameters) -> Mesh<BlockVertex> {
        let mut verts = Vec::new();
        let mut indices = Vec::new();

        let max_biome_height = (biome.base_height + biome.amplitude) as usize;

        zone!(" update chunk mesh"); // Span por hilo


        
        // Iterar solo sobre el área interna (1..CHUNK_AREA+1 para saltar el padding)
        for y in 0..CHUNK_Y_SIZE {
            for z in 1..=CHUNK_AREA {
                for x in 1..=CHUNK_AREA {

                    if y > max_biome_height {
                        continue;
                    }
                    zone!("procesing block vertices"); // Span por hilo

                    let block = blocks[y][x][z];
                    let mut block_vertices = Vec::with_capacity(4 * 6);
                    let mut block_indices: Vec<u16> = Vec::with_capacity(6 * 6);
                    
                    if block.material_type as i32 == MaterialType::AIR as i32 {
                        continue;
                    }

                    let mut quad_counter = 0;

                    for quad in block.quads.iter() {
                        let neighbor_pos: Vector3<i32> = block.get_vec_position() + quad.side.to_vec();
                        let visible = self.determine_visibility(&neighbor_pos, blocks);

                        if visible {
                            block_vertices.extend_from_slice(&quad.vertices);
                            block_indices.extend_from_slice(&quad.get_indices(quad_counter));
                            quad_counter += 1;
                        }
                    }
                    
                    block_indices = block_indices.iter().map(|i| i + verts.len() as u16).collect();
                    verts.extend(block_vertices);
                    indices.extend(block_indices);
                }
            }
        }

        Mesh { verts, indices }
    }


    /// Helper function to check visibility of a block
    fn determine_visibility(&self, neighbor_pos: &Vector3<i32>, blocks: &Blocks) -> bool {
        if ChunkArray::pos_in_chunk_bounds(*neighbor_pos) {
            // Convertir coordenadas (-1..16) a índices de array (0..17)

            let x_index = (neighbor_pos.x + 1) as usize;
            let y_index = neighbor_pos.y as usize;
            let z_index = (neighbor_pos.z + 1) as usize;
            
            let neighbor_block = blocks[y_index][x_index][z_index];
            return neighbor_block.material_type as u16 == MaterialType::AIR as u16;
        } else {
            return false;
        }
    }

    pub fn world_pos_in_bounds(&self, world_pos: Vector3<f32>) -> bool {
        let chunk_offset = Self::world_pos_to_chunk_offset(world_pos);
        self.chunk_in_bounds(chunk_offset)
    }





    // world array index -> chunk offset
    fn get_chunk_offset(&self, i: usize) -> Vector3<i32> {
        return self.chunks_origin + Vector3::new(i as i32 % CHUNKS_VIEW_SIZE as i32, 0, i as i32 / CHUNKS_VIEW_SIZE as i32);
    }


    fn chunk_in_bounds(&self, chunk_offset: Vector3<i32>) -> bool {
        let p = chunk_offset - self.chunks_origin;
        if p.x >= 0 && p.z >= 0 && p.x < CHUNKS_VIEW_SIZE as i32 && p.z < CHUNKS_VIEW_SIZE as i32 {
            return true;
        }
        return false;
    }


    fn world_pos_to_chunk_offset(world_pos: Vector3<f32>) -> Vector3<i32> {
        Vector3::new(
            (world_pos.x / CHUNK_AREA as f32).floor() as i32,
            0,
            (world_pos.z / CHUNK_AREA as f32).floor() as i32,
        )
    }
    
    fn get_chunk_world_index(&self, chunk_offset: Vector3<i32>) -> usize {
        let p = chunk_offset - self.chunks_origin;
        (p.z as usize * CHUNKS_VIEW_SIZE) + p.x as usize
    }




    //called every frame
    pub fn update(&mut self, queue: &Queue, player_position: &Point3<f32>) {

        zone!("update_world"); // <- Marca el inicio del bloque

        let new_center_offset = Self::world_pos_to_chunk_offset(player_position.to_vec());
        let new_chunk_origin = new_center_offset - Vector3::new(CHUNKS_VIEW_SIZE as i32 / 2, 0, CHUNKS_VIEW_SIZE as i32 / 2);

        if new_chunk_origin == self.chunks_origin {
            return;
        }

        self.center_offset = new_center_offset;
        self.chunks_origin = new_chunk_origin;
        //println!("chunks origin updated {:?}", self.chunks_origin);

        let chunk_indices_copy = self.chunk_indices.read().unwrap().clone();
        self.chunk_indices = Arc::new(RwLock::new([None; CHUNKS_ARRAY_SIZE]));

        for i in 0..CHUNKS_ARRAY_SIZE {
            match chunk_indices_copy[i] {
                Some(chunk_index) => {
                    let chunk_offset = self.chunks.offset_array.get(chunk_index).unwrap().read().unwrap().clone();
                    if self.chunk_in_bounds(chunk_offset.into()) {
                        let new_chunk_world_index = self.get_chunk_world_index(chunk_offset.into());
                        self.chunk_indices.write().unwrap()[new_chunk_world_index] = Some(chunk_index);
                    } else {
                        self.free_chunk_indices.write().unwrap().push_back(chunk_index);
                    }
                }
                None => {}
            }
        }

        self.load_empty_chunks(queue);
    }


}

impl Draw for World {
    fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, globals: &'a wgpu::BindGroup) -> Result<(), wgpu::Error> {

        zone!("drawing world"); // <- Marca el inicio del bloque

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.atlas.bind_group, &[]);
        render_pass.set_bind_group(1, globals, &[]);
        
        for chunk_model in &self.chunk_models {
                let vertex_buffer = chunk_model.vbuf().slice(..);
                let index_buffer = chunk_model.ibuf().slice(..);
                let num_indices = chunk_model.num_indices;

                render_pass.set_vertex_buffer(0, vertex_buffer);
                render_pass.set_index_buffer(index_buffer, wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..num_indices as u32, 0, 0..1 as _);
        }
        
        Ok(())
    }
}