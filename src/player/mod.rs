pub mod camera;
pub mod raycast;


pub struct Player {
    pub camera: camera::Camera,
    pub last_pos: cgmath::Point3<f32>,
    pub speed: f32,
    
}


impl Player {
    pub fn new(camera: camera::Camera) -> Self {
        Self {
            camera,
            last_pos: cgmath::Point3::new(0.0, 0.0, 0.0),
            speed: 0.1,
        }
    }
}