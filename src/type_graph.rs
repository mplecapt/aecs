use std::any::TypeId;

use hashbrown::{HashSet, HashMap};
use uuid::Uuid;

use crate::{archetype::Archetype, ArchetypeId, EntityId};

pub trait Position {
	fn types(&self) -> HashSet<TypeId>;
	fn element(&self) -> &Archetype;
	fn element_mut(&mut self) -> &mut Archetype;
}

struct Node {
	element: Archetype,
	types: HashSet<TypeId>,
	subsets: HashMap<TypeId, ArchetypeId>,	// parents
	supsets: HashMap<TypeId, ArchetypeId>,	// children
}
impl Position for Node {
	fn types(&self) -> HashSet<TypeId> {
		self.types.clone()
	}
	fn element(&self) -> &Archetype {
		&self.element
	}
	fn element_mut(&mut self) -> &mut Archetype {
		&mut self.element
	}
}
macro_rules! node_from {
	($($t:ty),+) => {
		Node {
			element: crate::archetype!($($t),+),
			types: HashSet::from([$(TypeId::of::<$t>()),+]),
			subsets: HashMap::new(),
			supsets: HashMap::new(),
		}
	};
}

pub struct TypeGraph {
	root: ArchetypeId,
	nodes: HashMap<ArchetypeId, Node>,
}
impl TypeGraph {
	pub fn new() -> Self {
		let root = ArchetypeId(Uuid::new_v4());
		let mut nodes = HashMap::new();
		nodes.insert(root, node_from!(EntityId));
		Self {
			root,
			nodes,
		}
	}
	pub fn root(&self) -> ArchetypeId { self.root }
	#[allow(dead_code)]
	pub fn positions<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Position> + 'a> {
		Box::new(self.nodes.values().map(|n| n as &dyn Position))
	}
	#[allow(dead_code)]
	pub fn positions_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn Position> + 'a> {
		Box::new(self.nodes.values_mut().map(|n| n as &mut dyn Position))
	}
	pub fn get_mut(&mut self, a_id: &ArchetypeId) -> Option<&mut Archetype> {
		if let Some(node) = self.nodes.get_mut(&a_id) {
			return Some(&mut node.element);
		}
		None
	}
	pub fn get(&self, a_id: &ArchetypeId) -> Option<&Archetype> {
		if let Some(node) = self.nodes.get(&a_id) {
			return Some(&node.element);
		}
		None
	}
	pub fn get_many<const W:usize>(&mut self, ids: [&ArchetypeId;W]) -> Option<[&mut Archetype;W]> {
		if let Some(nodes) = self.nodes.get_many_mut(ids) {
			Some(nodes.map(|n| &mut n.element))
		} else {
			None
		}
	}
	pub fn get_superset_with<T:'static>(&self, src: ArchetypeId) -> Option<ArchetypeId> {
		let src = self.nodes.get(&src).unwrap();
		src.supsets.get(&TypeId::of::<T>()).copied()
	}
	pub fn get_subset_without<T:'static>(&self, src: ArchetypeId) -> Option<ArchetypeId> {
		let src = self.nodes.get(&src).unwrap();
		src.subsets.get(&TypeId::of::<T>()).copied()
	}
	pub fn create_superset_with<T:'static>(&mut self, src: ArchetypeId) -> ArchetypeId {
		let sub = self.nodes.get(&src).unwrap();

		let mut element = sub.element.imitate();
		element.add_component::<T>();

		let mut types = sub.types.clone();
		types.insert(TypeId::of::<T>());

		let new_id = ArchetypeId(Uuid::new_v4());
		let mut subsets = HashMap::new();
		subsets.insert(TypeId::of::<T>(), src);
		let mut new_node = Node { element, types, subsets, supsets: HashMap::new() };
		self.connect_neighbors(new_id, &mut new_node);

		self.nodes.insert(new_id, new_node);
		new_id
	}
	pub fn create_subset_without<T:'static>(&mut self, src: ArchetypeId) -> ArchetypeId {
		let sup = self.nodes.get(&src).unwrap();

		let mut element = sup.element.imitate();
		element.remove_component::<T>();

		let mut types = sup.types.clone();
		types.remove(&TypeId::of::<T>());

		let new_id = ArchetypeId(Uuid::new_v4());
		let mut supsets = HashMap::new();
		supsets.insert(TypeId::of::<T>(), src);
		let mut new_node = Node { element, types, subsets: HashMap::new(), supsets };
		self.connect_neighbors(new_id, &mut new_node);

		self.nodes.insert(new_id, new_node);
		new_id
	}
	fn connect_neighbors(&mut self, a_id: ArchetypeId, target: &mut Node) {
		let defer: Vec<ArchetypeId> = self.nodes.iter().map(|e| e.0.clone()).collect();
		for cur in defer {
			let node = self.nodes.get_mut(&cur).unwrap();
			if target.types.len().abs_diff(node.types.len()) == 1 {
				if node.types.is_subset(&target.types) {
					for ty in target.types.difference(&node.types).cloned() {
						node.supsets.insert(ty, a_id);
						target.subsets.insert(ty, cur);
					}
				}
				if node.types.is_superset(&target.types) {
					for ty in node.types.difference(&target.types).cloned() {
						node.subsets.insert(ty, a_id);
						target.supsets.insert(ty, cur);
					}
				}
			}
		}
	}
}