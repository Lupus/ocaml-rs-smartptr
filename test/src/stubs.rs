use crate::animals;
use ctor::ctor;
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr::{register_trait, register_type};

extern crate derive_more;
use derive_more::AsRef;

// Animal bindings

// We have to introduce a proxy trait for animals::Animal, as animals::Animal
// is not object-safe because it has a ::new() static method, see
// https://doc.rust-lang.org/reference/items/traits.html#object-safety
// and https://www.possiblerust.com/pattern/3-things-to-try-when-you-can-t-make-a-trait-object
pub trait AnimalProxy {
    fn name(&self) -> String;
    fn noise(&self) -> String;
    fn talk(&self);
}

// In case multiple traits need to be composed into a trait object
// trait Composite: Trait1 + Trai2 {}
// impl<T> Composite for T where T: Trait1 + Trait2 {}
// use DynBox<dyn Composite + Send>

// could probably be generated with some macro TODO
// our AnimalProxy is automatically applicable to any type which implements
// animals::Animal
impl<T: animals::Animal> AnimalProxy for T {
    fn name(&self) -> String {
        self.name()
    }

    fn noise(&self) -> String {
        self.noise()
    }

    fn talk(&self) {
        self.talk()
    }
}

// Bindings use object-safe part of animals::Animal
pub type Animal = dyn AnimalProxy + Send;

#[ocaml_gen::func]
#[ocaml::func]
pub fn animal_name(animal: DynBox<Animal>) -> String {
    let animal = animal.coerce();
    animal.name()
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn animal_noise(animal: DynBox<Animal>) -> String {
    let animal = animal.coerce();
    animal.noise()
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn animal_talk(animal: DynBox<Animal>) {
    let animal = animal.coerce();
    animal.talk()
}

// Sheep bindings
pub type Sheep = animals::Sheep;

#[allow(dead_code)]
#[derive(AsRef)]
pub struct SheepWrapper(animals::Sheep);

#[ocaml_gen::func]
#[ocaml::func]
pub fn sheep_create(name: String) -> DynBox<Sheep> {
    let sheep: Sheep = animals::Animal::new(name);
    sheep.into()
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn sheep_is_naked(sheep: DynBox<Sheep>) -> bool {
    let sheep = sheep.coerce();
    sheep.is_naked()
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn sheep_sheer(sheep: DynBox<Sheep>) {
    let mut sheep = sheep.coerce_mut();
    sheep.shear()
}

// Wolf bindings
pub type Wolf = animals::Wolf;

#[ocaml_gen::func]
#[ocaml::func]
pub fn wolf_create(name: String) -> DynBox<Wolf> {
    let wolf: Wolf = animals::Animal::new(name);
    wolf.into()
}

// Register supported traits for types that we bind
#[ctor]
fn register_rtti() {
    register_trait!(
        {
            ty: crate::stubs::AnimalProxy,
            marker_traits: [core::marker::Sync, core::marker::Send],
        }
    );
    register_type!(
        {
            ty: crate::stubs::Sheep,
            marker_traits: [core::marker::Sync, core::marker::Send],
            object_safe_traits: [crate::stubs::AnimalProxy],
        }
    );
    register_type!(
        {
            ty: crate::stubs::Wolf,
            marker_traits: [core::marker::Sync, core::marker::Send],
            object_safe_traits: [crate::stubs::AnimalProxy],
        }
    );
}
