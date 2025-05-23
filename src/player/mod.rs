pub mod camera;
pub mod raycast;


pub struct Player {
    pub camera: camera::Camera,
    pub speed: f32,
    
}


impl Player {
    pub fn new(camera: camera::Camera) -> Self {
        Self {
            camera,
            speed: 0.1,
        }
    }
}