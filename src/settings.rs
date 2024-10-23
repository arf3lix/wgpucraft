use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::prelude::*;
use serde_json; 

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub graphics: GraphicsSettings,
    //pub audio: AudioSettings,
}




impl Default for Settings {
    fn default() -> Self {
        Self {
            graphics: GraphicsSettings::default(),
            //audio: AudioSettings::default(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GraphicsSettings {
    pub resolution: (u16, u16),
    pub fullscreen: bool,
}


impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution: (1920, 1080), // Default resolution
            fullscreen: false, // Default fullscreen setting
        }
    }
}

// #[derive(Serialize, Deserialize)]
// pub struct AudioSettings {
//     pub volume: u8,
//     pub mute: bool,
// }

// impl Default for AudioSettings {
//     fn default() -> Self {
//         Self {
//             volume: 75, // Default volume level
//             mute: false, // Default mute setting
//         }
//     }
// }

impl Settings {
    pub fn load_from_file(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let settings: Settings = serde_json::from_str(&contents)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let serialized = serde_json::to_string_pretty(self)?;
        let mut file = File::create(file_path)?;
        file.write_all(serialized.as_bytes())?;
        Ok(())
    }
}
