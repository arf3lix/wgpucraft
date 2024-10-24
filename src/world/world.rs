
use crate::{render::renderer::Renderer, world::biomes::{BiomeParameters, PRAIRIE_PARAMS}};


use std::{collections::VecDeque, sync::{Arc, RwLock}};

use crate::render::{atlas::{Atlas, MaterialType}, mesh::Mesh, model::DynamicModel, pipelines::terrain::{BlockVertex, TerrainPipeline}, renderer::Draw};
use crate::render::pipelines::GlobalsLayouts;
use crate::world::chunk::{self, generate_chunk, Blocks, ChunkArray, CHUNK_AREA, CHUNK_Y_SIZE};


use cgmath::{EuclideanSpace, Point3, Vector3};
use chunk::local_pos_to_world;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use wgpu::Queue;


pub const LAND_LEVEL: usize = 9;
pub const CHUNKS_VIEW_SIZE: usize = 12;
pub const CHUNKS_ARRAY_SIZE: usize = CHUNKS_VIEW_SIZE * CHUNKS_VIEW_SIZE;

use std::sync::atomic::{AtomicUsize, Ordering};

// Atomic counter to assign unique thread identifiers
static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(1);

// Thread-local variable to store the thread identifier
thread_local! {
    static THREAD_ID: usize = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
}

// Function to get the current thread identifier
fn get_thread_id() -> usize {
    THREAD_ID.with(|id| *id)
}



pub struct World {
    pipeline: wgpu::RenderPipeline,
    atlas: Atlas,
    pub chunks: ChunkArray,
    chunk_indices: Arc<RwLock<[Option<usize>; CHUNKS_ARRAY_SIZE]>>,
    free_chunk_indices: Arc<RwLock<VecDeque<usize>>>,
    //updated_indices: Arc<RwLock<[bool; CHUNKS_ARRAY_SIZE]>>,
    center_offset: Vector3<i32>,
    chunks_origin: Vector3<i32>,
    chunk_models: Vec<DynamicModel<BlockVertex>>


}


impl World {
    pub fn new(renderer: &Renderer) -> Self {
        let global_layouts = GlobalsLayouts::new(&renderer.device);
        let atlas = Atlas::new(&renderer.device, &renderer.queue, &global_layouts).unwrap();
        let mut chunk_models = vec![];
        let mut chunks:ChunkArray = ChunkArray::default();
        let chunk_indices: [Option<usize>; CHUNKS_ARRAY_SIZE] = [None; CHUNKS_ARRAY_SIZE];
        //let updated_indices = Arc::new(RwLock::new([false; CHUNKS_ARRAY_SIZE]));
        let mut free_chunk_indices = VecDeque::new();


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


        let world_pipeline = TerrainPipeline::new(
            &renderer.device,
            &global_layouts,
            shader,
            &renderer.config
        );


        let center_offset = Vector3::new(0, 0, 0);
        let chunks_origin = center_offset - Vector3::new(CHUNKS_VIEW_SIZE as i32 / 2, 0, CHUNKS_VIEW_SIZE as i32 / 2);


        let mut world = Self {
            pipeline: world_pipeline.pipeline,
            atlas,
            chunks,
            chunk_models,
            center_offset,
            chunks_origin,
            //updated_indices,
            chunk_indices: Arc::new(RwLock::new(chunk_indices)),
            free_chunk_indices: Arc::new(RwLock::new(free_chunk_indices))
        };


        println!("about to load first chunks");
        world.load_empty_chunks(&renderer.queue);


        world
    }







    pub fn load_empty_chunks(&mut self, queue: &Queue) {

        let chunks_to_update: Vec<usize> = (0..CHUNKS_ARRAY_SIZE)
            .filter(|&i| self.chunk_indices.read().unwrap()[i].is_none())
            .collect();



        
        // println!("chunks to update: {:?}", chunks_to_update);
        // let barrier = Arc::new(Barrier::new(chunks_to_update.len()));


        chunks_to_update.into_par_iter().for_each(|i| {
                //let c = Arc::clone(&barrier); // Clonar la referencia a la barrera para cada hilo
                let thread_id = get_thread_id();

                println!("thread_id {:?}", thread_id);

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
                        10,
                        &PRAIRIE_PARAMS
                    );
                    println!("just gen chunk at thread_id {:?}", thread_id);

                    self.chunk_indices.write().unwrap()[i] = Some(new_index);

                    // Esperar hasta que todos los hilos hayan llegado a este punto
                    //c .wait();
                    println!("about to calculate mesh at thread_id {:?}", thread_id);
                    let mesh = self.update_mesh(&self.chunks.blocks_array[new_index].read().unwrap(),new_index, thread_id, chunk_offset);
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

            // if self.updated_indices.read().unwrap()[i] {
            //     self.chunk_models[i].update(queue, &self.chunks.mesh_array[i].read().unwrap(), 0);
            //     self.updated_indices.write().unwrap()[i] = false;
            //     //println!("selected to update")
            // }
            
        });


        println!("---------------------------------");
    }




    pub fn update_mesh(&self, blocks: &Blocks, index: usize, thread_id:usize, chunk_offset:Vector3<i32>) -> Mesh<BlockVertex> {


        let mut verts =Vec::new();
        let mut indices = Vec::new();
        for y in 0.. CHUNK_Y_SIZE{
            for z in 0..CHUNK_AREA {
                for x in 0..CHUNK_AREA {
                    let block = blocks[y][x][z].lock().unwrap().clone();
                    let mut block_vertices = Vec::with_capacity(4 * 6);
                    let mut block_indices: Vec<u16> = Vec::with_capacity(6 * 6);
                    if block.material_type as i32 == MaterialType::AIR as i32 {
                        continue;
                    }


                    let mut quad_counter = 0;


                    for quad in block.quads.iter() {
                        let neighbor_pos: Vector3<i32> = block.get_vec_position() + quad.side.to_vec();
                        let visible = self.determine_visibility(&neighbor_pos, blocks, index, thread_id, chunk_offset);


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
    fn determine_visibility(&self, neighbor_pos: &Vector3<i32>,blocks: &Blocks, index: usize, thread_id:usize, chunk_offset:Vector3<i32>) -> bool {

        //println!("block neighbor pos: {:?}", neighbor_pos);
        if ChunkArray::pos_in_chunk_bounds(*neighbor_pos) {
            let neighbor_block = blocks[neighbor_pos.y as usize][neighbor_pos.x as usize][neighbor_pos.z as usize].lock().unwrap();
            return neighbor_block.material_type as u16 == MaterialType::AIR as u16;
        } else { //if not in bounds, it means that the neighbor block belongs to a different chunk


            // return  true;

            if neighbor_pos.y < 0 || neighbor_pos.y >= CHUNK_Y_SIZE as i32 {
                return true; //this should be true but set to false to do not render due unreachable neighbor blocks
            }


            if let Some(neighbor_block_type) = self.get_block_from_neighboring_chunk(*neighbor_pos, index, thread_id, chunk_offset) {
                return neighbor_block_type == MaterialType::AIR;
            }
            else {
                true //this should be true but set to false to do not render due unreachable neighbor blocks
            }
        }
    }

    pub fn world_pos_in_bounds(&self, world_pos: Vector3<f32>) -> bool {
        let chunk_offset = Self::world_pos_to_chunk_offset(world_pos);
        self.chunk_in_bounds(chunk_offset)
    }

    // Get the block from a neighboring chunk, if it exists.
    fn get_block_from_neighboring_chunk(&self, neighbor_pos: Vector3<i32>, _index: usize, thread_id:usize, chunk_offset_orig:Vector3<i32>) -> Option<MaterialType> {
        //println!("out of bounds condition  | at thread: {:?}", thread_id);

        
        //let chunk_offset = *self.chunks.offset_array[index].read().unwrap();
        let world_pos = local_pos_to_world(chunk_offset_orig.into(), neighbor_pos);

        // if !self.world_pos_in_bounds(world_pos) {
        //     return None;
        // }
        let neighbor_chunk_offset = Self::world_pos_to_chunk_offset(world_pos);
        let orig_index = self.get_chunk_world_index(chunk_offset_orig);

        if self.chunk_in_bounds(neighbor_chunk_offset) {

            let neighbor_chunk_index = self.get_chunk_world_index(neighbor_chunk_offset);
            
            if orig_index == neighbor_chunk_index {
                println!("original chunk_offset: {:?}                      | at thread: {:?}", chunk_offset_orig, thread_id);

                println!("neighbor pos from current chunk: {:?}  | at thread: {:?}", neighbor_pos, thread_id);
                println!("current chunk index:                               {:?} | at thread: {:?}", orig_index, thread_id);
                //println!("current chunk_offset: {:?}                      | at thread: {:?}", chunk_offset, thread_id);
                println!("world pos of neighbor: {:?}      | at thread: {:?}", world_pos, thread_id);
                println!("-----------------------------------------------------------");
                println!("neighbor_chunk_offset: {:?}            | at thread: {:?}", neighbor_chunk_offset, thread_id);
                //println!("current chunk index from its chunk offset:              {:?} | at thread: {:?}", self.get_chunk_world_index(chunk_offset.into()), thread_id);
                println!("neighbor_chunk_index:                               {:?} | at thread: {:?}", neighbor_chunk_index, thread_id);
                println!("error!!");

            }
    
            
    
    
            let local_pos = Vector3::new(
                world_pos.x as i32 - (neighbor_chunk_offset[0] * CHUNK_AREA as i32),
                world_pos.y as i32- (neighbor_chunk_offset[1] * CHUNK_Y_SIZE as i32),
                world_pos.z as i32- (neighbor_chunk_offset[2] * CHUNK_AREA as i32),
            );
    
            //println!("local pos of neighbor block relative to neighbor chunk: {:?} | at thread: {:?}", local_pos, thread_id);
    
            
    
    
            if ChunkArray::pos_in_chunk_bounds(local_pos) {
                let neighbor_block = *self.chunks.blocks_array[neighbor_chunk_index].read().unwrap()[local_pos.y as usize][local_pos.x as usize][local_pos.z as usize].lock().unwrap();
                return Some(neighbor_block.material_type);
            }

            
    

        } else {
            return None
        }



        None
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
        let new_center_offset = Self::world_pos_to_chunk_offset(player_position.to_vec());
        let new_chunk_origin = new_center_offset - Vector3::new(CHUNKS_VIEW_SIZE as i32 / 2, 0, CHUNKS_VIEW_SIZE as i32 / 2);

        if new_chunk_origin == self.chunks_origin {
            return;
        }

        self.center_offset = new_center_offset;
        self.chunks_origin = new_chunk_origin;
        println!("chunks origin updated {:?}", self.chunks_origin);

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

    // fn is_inner_chunk(&self, chunk_offset: Vector3<i32>) -> bool {
    //     let p = chunk_offset - self.chunks_origin;
    //     p.x > 0 && p.z > 0 && p.x < (CHUNKS_VIEW_SIZE as i32 - 1) && p.z < (CHUNKS_VIEW_SIZE as i32 - 1)
    // }



}

impl Draw for World {
    fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, globals: &'a wgpu::BindGroup) -> Result<(), wgpu::Error> {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.atlas.bind_group, &[]);
        render_pass.set_bind_group(1, &globals, &[]);
        
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