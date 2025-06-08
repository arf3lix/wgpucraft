use std::any::{Any, TypeId};
use std::collections::HashMap;

use super::{
    component::{
        Component,
        ComponentStorage
    },
    entity::Entity
};


pub struct World {
    next_entity_id: usize,
    entities: Vec<Entity>,
    components: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl World {

    pub fn new() -> Self {
        Self {
            next_entity_id: 0,
            entities: Vec::new(),
            components: HashMap::new(),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let entity = Entity::new(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.push(entity);
        entity
    }

    pub fn insert_component<T: Component + 'static>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        let storage = self.components
            .entry(type_id)
            .or_insert_with(|| Box::new(ComponentStorage::<T>::new()));

        if let Some(storage) = storage
            .downcast_mut::<ComponentStorage<T>>() {
            storage
            .insert(entity, component);
        }

    }

    pub fn get_component<T: Component + 'static>(&self, entity: Entity) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let storage = self.components
            .get(&type_id)?;
        let storage = storage
            .downcast_ref::<ComponentStorage<T>>()?;
        storage.get(entity)
    }


    pub fn remove_component<T: Component + 'static>(&mut self, entity: Entity) -> Option<T> {
        let type_id = TypeId::of::<T>();
        let types_storage = self.components
            .get_mut(&type_id)?;
        let storage = types_storage
            .downcast_mut::<ComponentStorage<T>>()?;
        storage.remove(entity)
    }



}


