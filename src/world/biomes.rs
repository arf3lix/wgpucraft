pub struct BiomeParameters {
    pub base_height: f32,
    pub frequency: f32,
    pub amplitude: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f32,
}

pub const PRAIRIE_PARAMS: BiomeParameters = BiomeParameters {
    base_height: 10.0,
    frequency: 0.05,
    amplitude: 7.0,
    octaves: 3,
    persistence: 0.05,
    lacunarity: 2.0,
};

pub const MOUNTAIN_PARAMS: BiomeParameters = BiomeParameters {
    base_height: 15.0,
    frequency: 0.03,
    amplitude: 35.0,
    octaves: 4,
    persistence: 0.05,
    lacunarity: 2.0,
};