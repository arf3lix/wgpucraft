
use crate::terrain_gen::block::Quad;


use super::{pipelines::terrain::BlockVertex, Vertex};


#[derive(Clone)]


/// Represents a vec-based mesh on the CPU
pub struct Mesh<V: Vertex> {
    pub verts: Vec<V>,
    pub indices: Vec<u16>
}


impl<V: Vertex> Mesh<V>
       
{
    /// Create a new `Mesh`.
    pub fn new() -> Self { Self { verts: Vec::new(), indices: Vec::new() } }

    /// Clear vertices, allows reusing allocated memory of the underlying Vec.
    pub fn clear(&mut self) { self.verts.clear(); }


    /// Get a slice referencing the vertices of this mesh.
    pub fn vertices(&self) -> &[V] { &self.verts }


    pub fn push(&mut self, vert: V) { self.verts.push(vert); }


    // new method to add indices
    pub fn push_indices(&mut self, indices: &[u16]) {
        self.indices.extend_from_slice(indices);
    }


    // returns the indices
    pub fn indices(&self) -> &[u16] {
        &self.indices
    }

    pub fn add_quad(&mut self, quad: &Quad)
        where Vec<V>: Extend<BlockVertex>
    {
        let base_index = self.verts.len() as u16;
        self.verts.extend(quad.vertices);
        self.indices.extend(&quad.get_indices(base_index));
    }


    pub fn iter_verts(&self) -> std::slice::Iter<V> { self.verts.iter() }


    pub fn iter_indices(&self) -> std::vec::IntoIter<u16> { self.indices.clone().into_iter() }


   
}







