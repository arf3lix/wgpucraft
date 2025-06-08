use super::universe::World;

pub trait System {
    /// Ejecuta la lógica del sistema.
    fn run(&self, world: &mut World);
}
