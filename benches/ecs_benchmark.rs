use aecs::{Manager, iter_over};
use rand::Rng;
use criterion:: {
	black_box,
	criterion_group,
	criterion_main,
	Criterion
};


#[derive(Clone)]
struct A(f32);
#[derive(Clone)]
struct B(f32);
#[derive(Clone)]
struct C(f32);

fn rand_manager() -> Manager {
	let mut man = Manager::new();
	let mut rng = rand::thread_rng();

	for _ in 0..1000 {
		let id = man.create_entity();
		match rng.gen_range(0..=7) {
			1 => man.attach_component(id, A(rng.gen::<f32>())),
			2 => man.attach_component(id, B(rng.gen::<f32>())),
			3 => man.attach_component(id, C(rng.gen::<f32>())),
			4 => {
				man.attach_component(id, A(rng.gen::<f32>()));
				man.attach_component(id, B(rng.gen::<f32>()));
			}
			5 => {
				man.attach_component(id, C(rng.gen::<f32>()));
				man.attach_component(id, B(rng.gen::<f32>()));
			}
			6 => {
				man.attach_component(id, C(rng.gen::<f32>()));
				man.attach_component(id, A(rng.gen::<f32>()));
			}
			7 => {
				man.attach_component(id, C(rng.gen::<f32>()));
				man.attach_component(id, B(rng.gen::<f32>()));
				man.attach_component(id, A(rng.gen::<f32>()));
			}
			_ => continue,
		}
	}

	man
}

fn sum(man: &mut Manager) {
	let mut a;
	let mut b;
	let mut c;
	let mut accum = (0., 0., 0.);
	for e in iter_over!(man, a:A, b:B, c:C) {
		accum.0 += e.0.0;
		accum.1 += e.1.0;
		accum.2 += e.2.0;
	}
}

fn ecs_benchmark(c: &mut Criterion) {
	let mut man = black_box(
		rand_manager()
	);

	c.bench_function(
		"iterations",
		|b| b.iter(|| sum(&mut man))
	);
}

criterion_group!(benches, ecs_benchmark);
criterion_main!(benches);