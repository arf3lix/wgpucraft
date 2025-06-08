/// Identificador único de una entidad en el mundo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(usize);

impl Entity {
    /// Crea una nueva entidad con un ID específico.
    /// (Nota: En la práctica, solo `World` debería poder crear entidades).
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Devuelve el ID numérico de la entidad.
    pub fn id(&self) -> usize {
        self.0
    }
}