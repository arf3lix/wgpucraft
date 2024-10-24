
use noise::{NoiseFn, Perlin};

pub struct NoiseGenerator {
    perlin: Perlin,

}

impl NoiseGenerator {
    pub fn new(seed: u32) -> Self {
        let perlin = Perlin::new(seed);
        Self { perlin}
    }

    pub fn get_height(&self, x: f32, z: f32, frequency: f32, amplitude: f32) -> f32 {
        self.perlin.get([x as f64 * frequency as f64, z as f64 * frequency as f64]) as f32 * amplitude
    }
}