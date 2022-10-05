use std::any::TypeId;

use hashbrown::HashMap;

use crate::component_vec::ComponentVec;

pub struct Archetype {
	components: HashMap<TypeId, ComponentVec>,
	entity_count: usize,
}
impl Archetype {
	pub fn new() -> Self {
		Self {
			components: HashMap::new(),
			entity_count: 0,
		}
	}
	pub fn len(&self) -> usize {
		self.entity_count
	}
	pub fn imitate(&self) -> Self {
		let mut new_comps = HashMap::new();
		for (tid, cv) in self.components.iter() {
			new_comps.insert(*tid, cv.imitate());
		}
		Self {
			components: new_comps,
			entity_count: 0,
		}
	}
	pub unsafe fn publish_push(&mut self) -> usize {
		let eid = self.entity_count;
		self.entity_count += 1;
		eid
	}
	pub unsafe fn push_partial<T:'static>(&mut self, component: T) {
		let cv = self.components.get_mut(&TypeId::of::<T>()).unwrap();
		cv.push(component);
	}
	pub fn remove_entity(&mut self, entity: usize) {
		for cv in self.components.values_mut() {
			cv.swap_forget(entity);
		}
	}
	pub fn upgrade_entity<T:'static>(&mut self, dst: &mut Self, entity: usize, component: T) -> usize {
		assert!((dst.components.len() - self.components.len()) == 1, "Invalid destination");
		assert!(self.entity_count > 0, "No entity to upgrade");
		for (tid, src_cv) in self.components.iter_mut() {
			if let Some(dst_cv) = dst.components.get_mut(&tid) {
				src_cv.swap_to_tail(entity);
				dst_cv.adopt_tail(src_cv);
			} else {
				panic!("Destination doesn't share type {:?}", tid);
			}
		}
		if let Some(dst_cv) = dst.components.get_mut(&TypeId::of::<T>()) {
			dst_cv.push(component);
		} else {
			panic!("Destination doesn't share type {:?}", TypeId::of::<T>());
		}
		self.entity_count -= 1;
		dst.entity_count += 1;
		dst.entity_count - 1
	}
	pub fn downgrade_entity(&mut self, dst: &mut Self, entity: usize) -> usize {
		assert!((self.components.len() - dst.components.len()) == 1, "Invalid destination");
		assert!(self.entity_count > 0, "No entity to downgrade");
		for (tid, src_cv) in self.components.iter_mut() {
			if let Some(dst_cv) = dst.components.get_mut(&tid) {
				src_cv.swap_to_tail(entity);
				dst_cv.adopt_tail(src_cv);
			}
		}
		self.entity_count -= 1;
		dst.entity_count += 1;
		dst.entity_count - 1
	}
	pub fn add_component<T:'static>(&mut self) -> &mut Self {
		assert!(self.entity_count == 0, "Cannot modify component list while occupied");
		self.components.insert(TypeId::of::<T>(), ComponentVec::new_as::<T>());
		self
	}
	pub fn remove_component<T:'static>(&mut self) -> &mut Self {
		assert!(self.entity_count == 0, "Cannot modify component list while occupied");
		self.components.remove(&TypeId::of::<T>());
		self
	}
	pub fn get_component<T:'static>(&self, entity: usize) -> Option<&T> {
		if let Some(cv) = self.components.get(&TypeId::of::<T>()) {
			Some(cv.as_slice()[entity])
		} else {
			None
		}
	}
	pub fn get_component_mut<T:'static>(&mut self, entity: usize) -> Option<&mut T> {
		if let Some(cv) = self.components.get_mut(&TypeId::of::<T>()) {
			Some(&mut cv.as_mut_slice()[entity])
		} else {
			None
		}
	}
	#[allow(dead_code)]
	pub fn get_component_vec<T:'static>(&self) -> &[T] {
		self.components.get(&TypeId::of::<T>()).unwrap().as_slice::<T>()
	}
	#[allow(dead_code)]
	pub fn get_component_vec_mut<T:'static>(&mut self) -> &mut [T] {
		self.components.get_mut(&TypeId::of::<T>()).unwrap().as_mut_slice::<T>()
	}
	#[allow(dead_code)]
	pub fn get_many_comp_vec_mut<const W:usize>(&mut self, types: [&TypeId;W]) -> [&mut ComponentVec;W] {
		self.components.get_many_mut(types).unwrap()
	}
}
#[macro_export]
macro_rules! archetype {
	($($t:ty),*) => {
		{
			let mut a = Archetype::new();
			$( a.add_component::<$t>(); )*
			a
		}
	};
}
#[macro_export]
macro_rules! push_entity {
	($a:expr, [$($comp:expr),+]) => {
		unsafe { 
			$( $a.push_partial($comp); )+ 
			$a.publish_push()
		}
	};
}