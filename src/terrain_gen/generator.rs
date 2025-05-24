use std::{collections::VecDeque, sync::{Arc, RwLock}};

use crate::{render::{atlas::Atlas, model::DynamicModel, pipelines::terrain::{create_terrain_pipeline, BlockVertex}, renderer::{Draw, Renderer}}, terrain_gen::biomes::PRAIRIE_PARAMS};
use crate::render::pipelines::GlobalsLayouts;
use crate::terrain_gen::chunk::{Chunk, ChunkManager, CHUNK_AREA, CHUNK_Y_SIZE};
use tracy::zone;


use cgmath::{EuclideanSpace, Point3, Vector3};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use wgpu::Queue;


pub const LAND_LEVEL: usize = 9;
pub const CHUNKS_VIEW_SIZE: usize = 16;
pub const CHUNKS_ARRAY_SIZE: usize = CHUNKS_VIEW_SIZE * CHUNKS_VIEW_SIZE;


use super::noise::NoiseGenerator;






pub struct TerrainGen {
    pipeline: wgpu::RenderPipeline,
    atlas: Atlas,
    pub chunks: ChunkManager,
    chunk_indices: Arc<RwLock<[Option<usize>; CHUNKS_ARRAY_SIZE]>>,
    free_chunk_indices: Arc<RwLock<VecDeque<usize>>>,
    center_offset: Vector3<i32>,
    chunks_origin: Vector3<i32>,
    pub chunk_models: Vec<Arc<RwLock<DynamicModel<BlockVertex>>>>,
    noise_gen: NoiseGenerator


}


impl TerrainGen {
    pub fn new(renderer: &Renderer) -> Self {
        let global_layouts = GlobalsLayouts::new(&renderer.device);
        let atlas = Atlas::new(&renderer.device, &renderer.queue, &global_layouts).unwrap();
        let mut chunk_models = vec![];
        let mut chunks = ChunkManager::new();
        let chunk_indices: [Option<usize>; CHUNKS_ARRAY_SIZE] = [None; CHUNKS_ARRAY_SIZE];
        //let updated_indices = Arc::new(RwLock::new([false; CHUNKS_ARRAY_SIZE]));
        let mut free_chunk_indices = VecDeque::new();

        let noise_gen = NoiseGenerator::new(10);



        for x in 0..CHUNKS_ARRAY_SIZE {
            //println!("initial x from new World: {:?}", x);
            chunks.add_chunk(Chunk::new([0,0,0]));
            let mut chunk_model = DynamicModel::new(&renderer.device, (CHUNK_AREA ^ 2) * CHUNK_Y_SIZE * 24);

            //TODO: handle unwraps
            chunk_model.update(&renderer.queue, &chunks.get_chunk(x).unwrap().read().unwrap().mesh, 0);
            chunk_models.push(Arc::new(RwLock::new(chunk_model)));
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
            //updated_indices,
            chunk_indices: Arc::new(RwLock::new(chunk_indices)),
            free_chunk_indices: Arc::new(RwLock::new(free_chunk_indices)),
            noise_gen
        };


        println!("about to load first chunks");
        world.load_empty_chunks(&renderer.queue);


        world
    }





    pub fn load_empty_chunks(&mut self, queue: &Queue) {
        zone!("load empty chunks"); // <- Marca el inicio del bloque

        let chunks_to_update: Vec<usize> = (0..CHUNKS_ARRAY_SIZE)
            .filter(|&i| self.chunk_indices.read().unwrap()[i].is_none())
            .collect();

        chunks_to_update.into_par_iter().for_each(|i| {
            zone!(" ldc: thread_work"); // Span por hilo

            let new_index = self.free_chunk_indices.write().unwrap().pop_front();
            if let Some(new_index) = new_index {
                let chunk_offset = self.get_chunk_offset(i);
                if !self.chunk_in_bounds(chunk_offset) {
                    panic!("Error: Cannot load chunk")
                }

                let chunk = self.chunks.get_chunk(new_index)
                    .expect("Error: Chunk not found");

                let mut chunk = chunk.write()
                    .expect("Error: Failed to lock chunk");
                
                chunk.update_blocks(chunk_offset.into(), &self.noise_gen, &PRAIRIE_PARAMS);
                
                self.chunk_indices.write().unwrap()[i] = Some(new_index);

                chunk.update_mesh(PRAIRIE_PARAMS);

                let mut chunk_model = self.chunk_models[new_index].write().unwrap();
                
                chunk_model.update(queue, &chunk.mesh, 0);


            } else {
                panic!("Error: No free space for chunk")
            }
        
        });


 

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
                    let chunk_offset = self.chunks.get_chunk(chunk_index).unwrap().read().unwrap().offset.clone();
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

impl Draw for TerrainGen {
    fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, globals: &'a wgpu::BindGroup) -> Result<(), wgpu::Error> {

        zone!("drawing world"); // <- Marca el inicio del bloque

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.atlas.bind_group, &[]);
        render_pass.set_bind_group(1, globals, &[]);
        
        for chunk_model in &self.chunk_models {
            let chunk_model = chunk_model.read().unwrap();
        
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