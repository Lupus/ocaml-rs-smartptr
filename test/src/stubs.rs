use crate::animals;
use ocaml_rs_smartptr::func::OCamlFunc;
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr::{
    ocaml_gen_bindings, register_rtti, register_trait, register_type,
};

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

#[ocaml_gen::func]
#[ocaml::func]
pub fn wolf_set_hungry(wolf: DynBox<Wolf>, hungry: bool) {
    let mut wolf = wolf.coerce_mut();
    wolf.set_hungry(hungry);
}

// Boxed trait bindings

#[ocaml_gen::func]
#[ocaml::func]
pub fn animal_create_random(name: String) -> DynBox<Animal> {
    let sheep: Sheep = animals::Animal::new(name);
    let animal: Box<Animal> = Box::new(sheep);
    DynBox::new_exclusive_boxed(animal)
}

// OCamlFunc bindings

#[ocaml_gen::func]
#[ocaml::func]
pub fn call_cb(
    wolf: DynBox<Wolf>,
    cb: OCamlFunc<(DynBox<Wolf>,), DynBox<Animal>>,
) -> DynBox<Animal> {
    /* Check that doing funny things with clones of OCamlFunc do not explode
     * boxroots */
    let cb2 = cb.clone();
    drop(cb);
    let res = cb2.call(gc, (wolf,));
    drop(cb2.clone());
    drop(cb2);
    res
}

// ocaml_export!  bindings

#[derive(ocaml::ToValue, ocaml::FromValue, ocaml_gen::CustomType)]
pub struct Barn {
    size: u32,
}

type DynBoxWithAnimal = DynBox<dyn AnimalProxy + Send + Sync>;

pub mod exports {
    ocaml_rs_smartptr::ocaml_export!(crate::stubs::Barn, Barn, "Some_other_lib.Barn.t");
    ocaml_rs_smartptr::ocaml_export!(
        crate::stubs::DynBoxWithAnimal,
        DynBoxWithAnimal,
        "Some_other_lib.Animal.t"
    );
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn barn_create(size: u32) -> exports::Barn {
    Barn { size }.into()
}

#[ocaml_gen::func]
#[ocaml::func]
pub fn dynbox_with_animal_create(name: String) -> exports::DynBoxWithAnimal {
    let wolf: Wolf = animals::Animal::new(name);
    let animal: Box<dyn AnimalProxy + Send + Sync> = Box::new(wolf);
    DynBox::new_exclusive_boxed(animal).into()
}

// Register types & traits
register_rtti! {
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

// OCaml bindings generation
ocaml_gen_bindings! {
    decl_module!("Animal", {
        decl_type!(DynBox<Animal> => "t");
        decl_func!(animal_name => "name");
        decl_func!(animal_noise => "noise");
        decl_func!(animal_talk => "talk");
        decl_func!(animal_create_random => "create_random");
    });

    decl_module!("Sheep", {
        decl_type!(DynBox<Sheep> => "t");
        decl_func!(sheep_create => "create");
        decl_func!(sheep_is_naked => "is_naked");
        decl_func!(sheep_sheer => "sheer");
    });

    decl_module!("Wolf", {
        decl_type!(DynBox<Wolf> => "t");
        decl_func!(wolf_create => "create");
        decl_func!(wolf_set_hungry => "set_hungry");
    });

    decl_module!("Test_callback", {
        decl_func!(call_cb => "call_cb");
    });

    decl_module!("Animal_alias", {
        decl_type_alias!("animal" => DynBox<Animal>);
        decl_func!(animal_create_random => "create_random_animal");
    });

    decl_module!("Export_import", {
        decl_func!(barn_create => "barn_create");
        decl_type_alias!("barn" => exports::Barn);
        decl_func!(barn_create => "barn_create_with_alias");
        decl_func!(dynbox_with_animal_create => "dynbox_with_animal_create");
    });
}
