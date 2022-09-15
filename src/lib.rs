use std::{any::TypeId, cell::{RefCell, RefMut}};

#[macro_use] extern crate mopa;
use hashbrown::HashMap;
use uuid::Uuid;

pub trait ComponentVec: mopa::Any {
	fn push_none(&self);
	fn swap_remove(&self, row: usize);
}
mopafy!(ComponentVec);
impl<T: 'static> ComponentVec for RefCell<Vec<Option<T>>> {
	fn push_none(&self) {
		self.borrow_mut().push(None);
	}
	fn swap_remove(&self, row: usize) {
		self.borrow_mut().swap_remove(row);
	}
}

#[macro_export]
macro_rules! iter_over {
	($a:expr, $t:ty) => ( $a.borrow_component_vec::<$t>().unwrap().iter_mut().filter_map(|c|Some(c.as_mut()?)) );
	// ($a:expr, $c1:ident:$t1:ty, $c2:ident:$t2:ty) => {{
	// 	$c1 = $a.borrow_component_vec::<$t1>().unwrap();
	// 	$c2 = $a.borrow_component_vec::<$t2>().unwrap();
	// 	let zip = $c1.iter_mut().zip($c2.iter_mut());
	// 	zip.filter_map(|(lh, rh)|Some((lh.as_mut()?, rh.as_mut()?)))
	// }};
	($a:expr, $($c:ident : $t:ty),*) => {{
		$( $c = $a.borrow_component_vec::<$t>().unwrap(); )*
		itertools::izip!( $( $c.iter_mut() ),* ).filter_map( |( $($c),* )| Some(( $($c.as_mut()?),* )) )
	}};
}


#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug)]
pub struct EntityId(Uuid);

pub struct Manager {
	entity_count: usize,
	components: Vec<Box<dyn ComponentVec>>,
	entity_index: HashMap<EntityId, usize>,
	type_index: HashMap<TypeId, usize>,
}
impl Manager {
	pub fn new() -> Self {
		let mut type_index = HashMap::new();
		type_index.insert(TypeId::of::<EntityId>(), 0);
		let entity_vec: RefCell<Vec<Option<EntityId>>> = RefCell::new(Vec::new());
		Self {
			entity_count: 0,
			components: vec![Box::new(entity_vec)],
			entity_index: HashMap::new(),
			type_index,
		}
	}
	fn gen_e_id(&self) -> EntityId {
		let mut id = EntityId(Uuid::new_v4());
		if let Some(_) = self.entity_index.get(&id) { id.0 = Uuid::new_v4(); }
		id
	}
	pub fn create_entity(&mut self) -> EntityId {
		let id = self.gen_e_id();
		self.borrow_id_vec().push(Some(id));
		for cv in self.components.iter().skip(1) {
			cv.push_none();
		}
		self.entity_index.insert(id, self.entity_count);
		self.entity_count += 1;
		id
	}
	pub fn destroy_entity(&mut self, entity: EntityId) {
		if let Some(row) = self.entity_index.get(&entity).cloned() {
			for cv in self.components.iter() {
				cv.swap_remove(row);
			}
			if row != self.entity_count {
				let updated = self.borrow_id_vec()[row].unwrap();
				self.entity_index.insert(updated, row);
			}
			self.entity_count -= 1;
		}
	}
	pub fn attach_component<CompType: 'static + Clone>(&mut self, entity: EntityId, component: CompType) {
		if let Some(row) = self.entity_index.get(&entity).cloned() {
			let comp_type = TypeId::of::<CompType>();
			let col = if let Some(col) = self.type_index.get(&comp_type).cloned() {
				col
			} else {
				let col = self.components.len();
				let cv: Vec<Option<CompType>> = vec![None; self.entity_count];
				self.components.push(Box::new(RefCell::new(cv)));
				self.type_index.insert(comp_type, col);
				col
			};
			let mut cv = self.components[col].downcast_ref::<RefCell<Vec<Option<CompType>>>>().unwrap().borrow_mut();
			cv[row] = Some(component);
		}
	}
	pub fn detach_component<CompType: 'static>(&self, entity: EntityId) {
		if let Some(row) = self.entity_index.get(&entity).cloned() {
			if let Some(mut cv) = self.borrow_component_vec::<CompType>() {
				cv[row] = None;
			}
		}
	}
	pub fn borrow_id_vec(&self) -> RefMut<Vec<Option<EntityId>>> {
		self.components[0].downcast_ref::<RefCell<Vec<Option<EntityId>>>>().unwrap().borrow_mut()
	}
	pub fn borrow_component_vec<CompType: 'static>(&self) -> Option<RefMut<Vec<Option<CompType>>>> {
		if let Some(col) = self.type_index.get(&TypeId::of::<CompType>()).cloned() {
			return Some(self.components[col].downcast_ref::<RefCell<Vec<Option<CompType>>>>().unwrap().borrow_mut());
		}
		None
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn manager() {
		let m = Manager::new();
		assert_eq!(m.entity_count, 0);
		assert_eq!(m.components.len(), 1);
		let s = m.borrow_component_vec::<EntityId>().unwrap();
		assert_eq!(s.len(), 0);
	}

	#[test]
	fn create_destroy_entity() {
		let mut m = Manager::new();

		let e1 = m.create_entity();
		let e2 = m.create_entity();
		let e3 = m.create_entity();
		assert_eq!(m.entity_count, 3);

		let s = m.borrow_id_vec().clone();
		assert_eq!(s, vec![Some(e1), Some(e2), Some(e3)]);

		m.destroy_entity(e1);
		let s = m.borrow_id_vec().clone();
		assert_eq!(s, vec![Some(e3), Some(e2)]);
		assert_eq!(m.entity_index[&e3], 0);
	}

	#[derive(Clone, PartialEq, Debug)]
	struct Pos	(f32, f32);
	#[derive(Clone, PartialEq, Debug)]
	struct Name	(&'static str);
	#[derive(Clone, PartialEq, Debug)]
	struct Tri { points: [usize;3]}
	#[allow(dead_code)]
	impl Tri {
		fn draw(&self) -> [usize;3] { return self.points; }
	}

	macro_rules! clone_cv {
		($m:expr, $t:ty) => {
			$m.borrow_component_vec::<$t>().unwrap().clone()
		};
	}

	macro_rules! assert_comp_vec {
		($m:expr, $t:ty[$($c:expr),*]) => {
			assert_eq!(clone_cv!($m, $t), vec![$($c),*]);
		};
	}

	#[test]
	fn attach_detach_component() {
		let mut m = Manager::new();

		let e1 = m.create_entity();
		let e2 = m.create_entity();
		let e3 = m.create_entity();

		m.attach_component(e1, Pos(0., 0.));
		assert_comp_vec!(m, Pos[ Some(Pos(0., 0.)), None, None ]);
		m.attach_component(e1, Pos(1., 0.));
		m.attach_component(e3, Pos(-1., -3.));
		assert_comp_vec!(m, Pos[ Some(Pos(1., 0.)), None, Some(Pos(-1., -3.)) ]);

		m.attach_component(e2, Name("Entity 2"));
		m.attach_component(e3, Name("Entity 3"));
		assert_comp_vec!(m, Name[ None, Some(Name("Entity 2")), Some(Name("Entity 3")) ]);

		m.attach_component(e2, Tri { points: [0, 0, 0] });
		assert_comp_vec!(m, Tri[ None, Some(Tri { points: [0, 0, 0] }), None ]);

		m.detach_component::<Name>(e2);
		assert_comp_vec!(m, Name[ None, None, Some(Name("Entity 3")) ]);
	}

	#[test]
	fn iter_components() {
		let mut m = Manager::new();

		let e1 = m.create_entity();
		m.attach_component(e1, Name("Entity 1"));
		let e2 = m.create_entity();
		m.attach_component(e2, Name("Entity 2"));
		let e3 = m.create_entity();
		m.attach_component(e3, Name("Entity 3"));
		m.attach_component(e1, Pos(1., 1.));
		m.attach_component(e3, Pos(3., 3.));
		m.attach_component(e3, Tri { points: [0, 0, 0] });
		assert_comp_vec!(m, Name[ Some(Name("Entity 1")), Some(Name("Entity 2")), Some(Name("Entity 3")) ]);
		assert_comp_vec!(m, Pos[ Some(Pos(1.,1.)), None, Some(Pos(3., 3.)) ]);
		assert_comp_vec!(m, Tri[ None, None, Some(Tri { points: [0, 0, 0] }) ]);
		
		let b = iter_over!(m, Pos).count();
		assert_eq!(b, 2);
		
		
		{
			let mut name;
			let mut pos;
			for entity in iter_over!{m, name: Name, pos: Pos} {
				match entity {
					(n, p) if n.clone() == Name("Entity 1") => { assert_eq!(p.clone(), Pos(1., 1.)); },
					(n, p) if n.clone() == Name("Entity 3") => { assert_eq!(p.clone(), Pos(3., 3.)); },
					_ => { assert!( false, "Shouldn't reach here"); }
				}
			}
		}
		
		{
			let mut name;
			let mut pos;
			let mut tri;
			for entity in iter_over!(m, name:Name, pos:Pos, tri:Tri) {
				assert_eq!(entity.0.clone(), Name("Entity 3"));
				assert_eq!(entity.1.clone(), Pos(3., 3.));
				assert_eq!(entity.2.clone(), Tri { points: [0, 0, 0] });
			}
		}
	}
}