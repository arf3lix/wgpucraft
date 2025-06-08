use std::any::{Any, TypeId};
use std::collections::HashMap;

use super::entity::Entity;



pub trait  Component: Any + Send + Sync {

    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    
}


/// Macro para derivar automáticamente el trait `Component`.
/// Esto permite usar `#[derive(Component)]` en structs.
#[macro_export]
macro_rules! derive_component {
    ($t:ty) => {
        impl Component for $t {}
    };
}


pub struct ComponentStorage<T> {
    components: HashMap<Entity, T>
}

impl<T: Component> ComponentStorage<T> {

    pub fn new() -> Self {
        Self {
            components: HashMap::new()
        }
    }

    pub fn insert(&mut self, entity: Entity, component: T) {
        self.components.insert(entity, component);
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.components.get(&entity)
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        self.components.remove(&entity)
    }

    pub fn has_entity(&mut self, entity: Entity) -> bool {
        self.components.contains_key(&entity)
    }
}