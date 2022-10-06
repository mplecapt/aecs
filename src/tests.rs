use super::*;

trait Letter {
	fn val(&self) -> char;
}

#[derive(Debug, PartialEq, Eq)]
struct A(usize);
impl Letter for A {
	fn val(&self) -> char {
		self.0 as u8 as char
	}
}

#[derive(Debug, PartialEq)]
struct B(f32);

#[derive(Debug, PartialEq, Eq)]
struct C(i64);
impl Letter for C {
	fn val(&self) -> char {
		self.0 as u8 as char
	}
}

fn basic_ecs() -> (ECS, [EntityId;4]) {
	let mut ecs = ECS::new();
	let _e1 = create_entity_from!(ecs, [A(10), B(-5.0)]);
	let _e2 = create_entity_from!(ecs, [A(5), C(100)]);
	let _e3 = create_entity_from!(ecs, [B(3.14), C(-4)]);
	let _e4 = create_entity_from!(ecs, [A(0), B(0.), C(0)]);
	(ecs, [_e1, _e2, _e3, _e4])
}

#[test]
fn iter() {
	let (ecs, entities) = basic_ecs();

	for res in iter_components!(ecs, EntityId, A, B ) {
		match res {
			(e, a,b) if e == &entities[0] => assert_eq!((a,b), (&A(10), &B(-5.0))),
			(e, a, b) if e == &entities[3] => assert_eq!((a,b), (&A(0), &B(0.))),
			_ => assert!(false, "Invalid entity"),
		}
	}
}

#[test]
fn iter_mut() {
	let (mut ecs, entities) = basic_ecs();

	{
		for (e, c) in iter_components_mut!(ecs, EntityId, C) {
			if e == &entities[1] {
				*c = C(-10);
			}
		}
	}
	{
		for res in iter_components!(ecs, EntityId, C) {
			match res {
				(e,c) if e == &entities[1] => assert_eq!(c, &C(-10)),
				(e,c) if e == &entities[2] => assert_eq!(c, &C(-4)),
				(e,c) if e == &entities[3] => assert_eq!(c, &C(0)),
				_ => assert!(false, "Invalid entry"),
			}
		}
	}
}

#[test]
fn iter_cast() {
	let (ecs, _entities) = basic_ecs();

	let x = iter_components_cast!(ecs, [A, C] as Letter).collect::<Vec<&dyn Letter>>();
	let x = &x[..];
}