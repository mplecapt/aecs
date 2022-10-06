mod component_vec;
mod archetype;
mod type_graph;
#[cfg(test)]
mod tests;

use std::any::TypeId;
use hashbrown::{HashMap, HashSet};
use uuid::Uuid;
use type_graph::TypeGraph;

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct EntityId(Uuid);

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct ArchetypeId(Uuid);

pub struct ECS {
	entity_index: HashMap<EntityId, (ArchetypeId, usize)>,
	component_index: HashMap<TypeId, HashSet<ArchetypeId>>,
	archetypes: TypeGraph,
}
impl ECS {
	pub fn new() -> Self {
		let archetypes = TypeGraph::new();
		let mut component_index = HashMap::new();
		component_index.insert(TypeId::of::<EntityId>(), HashSet::from([archetypes.root()]));
		Self {
			entity_index: HashMap::new(),
			component_index,
			archetypes,
		}
	}
	pub fn create_entity(&mut self) -> EntityId {
		let eid = EntityId(Uuid::new_v4());
		let row = push_entity!(self.archetypes.get_mut(&self.archetypes.root()).unwrap(), [eid]);
		self.entity_index.insert(eid, (self.archetypes.root(), row));
		eid
	}
	pub fn destroy_entity(&mut self, entity: EntityId) {
		if let Some((a_id, row)) = self.entity_index.get(&entity).cloned() {
			let arche = self.archetypes.get_mut(&a_id).unwrap();
			arche.remove_entity(row);
			// update references if swapped with end
			self.check_swapped_row(row, a_id);
		}
	}
	pub fn attach_component<CompType: 'static>(&mut self, entity: EntityId, component: CompType) {
		if let Some((old_id, old_row)) = self.entity_index.get(&entity).copied() {
			// find new archetype
			let new_id = if let Some(new_arche) = self.archetypes.get_superset_with::<CompType>(old_id) {
				// archetype exists
				new_arche
			} else {
				// create new archetype
				let new_arche = self.archetypes.create_superset_with::<CompType>(old_id);
				self.update_component_index(TypeId::of::<CompType>(), new_arche);
				new_arche
			};
			// move to new archetype
			{
				let [old_arche, new_arche] = self.archetypes.get_many([&old_id, &new_id]).unwrap();
				let new_row = old_arche.upgrade_entity(new_arche, old_row, component);
				self.entity_index.insert(entity, (new_id, new_row));
			}
			// update other affected rows
			self.check_swapped_row(old_row, old_id);
		}
	}
	pub fn detach_component<CompType: 'static>(&mut self, entity: EntityId) {
		if let Some((old_id, old_row)) = self.entity_index.get(&entity).copied() {
			let new_id = if let Some(new_id) = self.archetypes.get_subset_without::<CompType>(old_id) {
				new_id
			} else {
				let new_id = self.archetypes.create_subset_without::<CompType>(old_id);
				self.update_component_index(TypeId::of::<CompType>(), new_id);
				new_id
			};
			{
				let [old_arche, new_arche] = self.archetypes.get_many([&old_id, &new_id]).unwrap();
				let new_row = old_arche.downgrade_entity(new_arche, old_row);
				self.entity_index.insert(entity, (new_id, new_row));
			}
			self.check_swapped_row(old_row, old_id);
		}
	}
	pub fn has_component<T:'static>(&self, entity: EntityId) -> bool {
		let tid = TypeId::of::<T>();
		if let Some(types) = self.component_index.get(&tid) {
			if let Some((aid, _)) = self.entity_index.get(&entity) {
				return types.contains(aid);
			}
		}
		false
	}
	pub fn get_component<T:'static>(&self, entity: EntityId) -> Option<&T> {
		if let Some((a_id, row)) = self.entity_index.get(&entity).copied() {
			let a = self.archetypes.get(&a_id).unwrap();
			return a.get_component::<T>(row);
		}
		None
	}
	pub fn get_component_mut<T:'static>(&mut self, entity: EntityId) -> Option<&mut T> {
		if let Some((a_id, row)) = self.entity_index.get(&entity).copied() {
			let a = self.archetypes.get_mut(&a_id).unwrap();
			return a.get_component_mut::<T>(row);
		}
		None
	}
	fn check_swapped_row(&mut self, new_spot: usize, a_id: ArchetypeId) {
		let arche = self.archetypes.get(&a_id).unwrap();
		if new_spot != arche.len() {
			let moved = arche.get_component::<EntityId>(new_spot).copied().unwrap();
			self.entity_index.insert(moved, (a_id, new_spot));
		}
	}
	fn update_component_index(&mut self, tid: TypeId, new_arche: ArchetypeId) {
		if let Some(set) = self.component_index.get_mut(&tid) {
			set.insert(new_arche);
		} else {
			self.component_index.insert(tid, HashSet::from([new_arche]));
		}
	}
}

#[macro_export]
macro_rules! create_entity_from {
	($ecs:expr, [$($comp:expr),*]) => {
		{
			let eid = $ecs.create_entity();
			$(
				$ecs.attach_component(eid, $comp);
			)*
			eid
		}
	};
}

#[macro_export]
macro_rules! iter_components {
	($ecs:expr, $($t:ty),+) => {
		paste::paste! {
			$ecs.archetypes.positions().filter_map(|node| {
				if node.types().is_superset(&HashSet::from([$(TypeId::of::<$t>()),+])) {
					let arche = node.element();
					Some(( $(arche.get_component_vec::<$t>()),+ ))
				} else {
					None
				}
			}).flat_map(|( $([< $t:snake:lower >]),+ )| itertools::izip!($( [< $t:snake:lower >].iter() ),+))
		}
	}
}

#[macro_export]
macro_rules! iter_components_mut {
	($ecs:expr, $($t:ty),+) => {
		paste::paste! {
			$ecs.archetypes.positions_mut().filter_map(|node| {
				if node.types().is_superset(&HashSet::from([$(TypeId::of::<$t>()),+])) {
					let [$([< $t:snake:lower >]),+] = node.element_mut().get_many_comp_vec_mut([$(&TypeId::of::<$t>()),+]);
					Some(( $( [<$t:snake:lower>].as_mut_slice::<$t>() ),+ ))
				} else {
					None
				}
			}).flat_map(|( $([<$t:snake:lower>]),+)| itertools::izip!($([<$t:snake:lower>].iter_mut()),+))
		}
	}
}

#[macro_export]
macro_rules! iter_components_cast {
	($ecs:expr, [$first:ty, $($t:ty),*] as $cast:path) => {
		$ecs.archetypes.positions().filter_map(|node| {
			if node.types().contains(&TypeId::of::<$first>()) {
				Some(node.element().get_component_vec::<$first>().iter().map(|c| c as &dyn $cast).collect::<Vec<&dyn $cast>>())
			} else {
				None
			}
		})
		$(
			.chain($ecs.archetypes.positions().filter_map(|node| {
				if node.types().contains(&TypeId::of::<$t>()) {
					Some(node.element().get_component_vec::<$t>().iter().map(|c| c as &dyn $cast).collect::<Vec<&dyn $cast>>())
				} else {
					None
				}
			}))
		)*
		.flat_map(|x| x.into_iter())
	}
}