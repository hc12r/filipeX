use super::{
    object::{Object, ObjectInfo},
    type_system::Type,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug, Clone)]
pub enum ContextType {
    Global,
    Function,
    Loop,
    IfElse,
}

#[derive(Debug, Clone)]
pub struct Context {
    type_: ContextType,
    store: HashMap<String, ObjectInfo>,
    parent: Option<Rc<RefCell<Context>>>,
}

impl Context {
    pub fn make_from(parent: Rc<RefCell<Context>>, type_: ContextType) -> Self {
        Self {
            type_,
            store: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn make_global(store: HashMap<String, ObjectInfo>) -> Self {
        Self {
            type_: ContextType::Global,
            store,
            parent: None,
        }
    }

    pub fn set(&mut self, name: String, type_: Type, value: Object, is_assignable: bool) -> bool {
        if self.store.contains_key(&name) {
            return false;
        }
        self.store.insert(
            name,
            ObjectInfo {
                value,
                is_assignable,
                type_,
            },
        );
        true
    }

    pub fn mutate(&mut self, name: String, value: Object) -> bool {
        if self.store.contains_key(&name) {
            let old = self.store.get_mut(&name).unwrap();
            if !old.is_assignable {
                return false;
            }
            old.value = value;
            return true;
        }
        match self.parent {
            Some(ref p) => p.borrow_mut().mutate(name, value),
            None => false,
        }
    }

    pub fn has(&self, name: &str) -> bool {
        self.store.contains_key(name)
    }

    pub fn resolve(&self, name: &str) -> Option<ObjectInfo> {
        if self.store.contains_key(name) {
            let obj = self.store.get(name).unwrap();
            return Some(obj.clone());
        }
        match self.parent {
            Some(ref p) => p.borrow().resolve(name),
            None => None,
        }
    }
}
