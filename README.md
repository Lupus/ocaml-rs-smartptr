# Smart Pointer for ocaml-rs

**WARNING**: Highly experimental code, do not use in production!

## Overview

This project provides a smart pointer implementation for `ocaml-rs`, allowing
for safe interaction between Rust and OCaml. The library includes various
utilities and extensions for generating OCaml bindings, handling OCaml
functions, and managing OCaml values in Rust.

## Modules

### `src/ptr.rs`

- **DynBox**: A smart pointer type for safe and flexible interop between OCaml and Rust.

### `src/ml_box.rs`

- **MlBox**: A wrapper around `ocaml::Value` that allows to safely pass it between threads from Rust.

### `src/func.rs`

- **OCamlFunc**: A wrapper around `MlBox` representing an OCaml function. It ensures safe calls from Rust.
- **OCamlDesc Implementation**: Provides OCaml type descriptions for functions.

### `src/ocaml_gen_extras.rs`

- **PolymorphicValue**: A wrapper around `ocaml::Value` printed as an OCaml polymorphic type.
- **TypeParams Trait**: Represents type parameters for generic types.
- **WithTypeParams**: A thin wrapper around a type with type parameters.
- **OcamlGenPlugin**: Represents a plugin for generating OCaml bindings.

## Usage

### Write Rust bindings to OCaml

Our bindings would rely on wonderful crates [ocaml-rs](https://github.com/zshipko/ocaml-rs) and [ocaml-gen](https://github.com/o1-labs/ocaml-gen).

```rust
// Bindings use object-safe part of animals::Animal
// see test/src/stubs.rs for complete sources
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

// OCamlFunc bindings

#[ocaml_gen::func]
#[ocaml::func]
pub fn call_cb(
    wolf: DynBox<Wolf>,
    cb: OCamlFunc<(DynBox<Wolf>,), DynBox<Animal>>,
) -> DynBox<Animal> {
    let res = cb.call(gc, (wolf,));
    res
}
```

### Registering Types and Traits

Use the provided macros to register types and traits for OCaml:

```rust
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
```

`register_trait` registeres an object-safe trait within the type registry, along
with all its combinations when "multiplied" by marker traits.

`register_type` registeres type, and coercions from that type to combinations of object-safe traits, "multiplied" by marker traits.

All this is required to force Rust to generate vtables and record convertion
functions between original type and a combination of traits.

### Declare OCaml Bindings

Use the `ocaml_gen_bindings` macro to declare OCaml bindings:

```rust
ocaml_gen_bindings! {
    decl_module!("Animal", {
        decl_type!(DynBox<Animal> => "t");
        decl_func!(animal_name => "name");
        decl_func!(animal_noise => "noise");
        decl_func!(animal_talk => "talk");
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
}
```

`ocaml_gen_bindings` declares more convenient aliases for
`ocaml_gen::decl_module` & co without extra boilerplate params (writeable string
and environment are managed by `ocaml_gen_bindings` internally). `DynBox<T>`
supports `ocaml_gen` infrastructure as long as `T` supports it.

### Generating OCaml bindings

You need a binary like this to generate the bindings:

```rust
#[allow(clippy::single_component_path_imports)]
#[allow(unused_imports)]
use ocaml_rs_smartptr_test;

fn main() -> std::io::Result<()> {
    ocaml_rs_smartptr::ocaml_gen_extras::stubs_gen_main()
}
```

Unused import is required for Cargo/Rust to actually link in the
`ocaml_rs_smartptr_test` library, allowing the plugin system (on top of
`inventory` crate) to register itself.

You can run this binary from a dune rule:

```
(rule
 (alias runtest)
 (targets Ocaml_rs_smartptr_test.ml)
 (deps stubs-gen.rs)
 (locks cargo-build)
 (action
  (run cargo run --offline --package stubs-gen --bin stubs-gen)))
```

This binary will generate one .ml file for each crate that declared the bindings
(and was linked in...).

### How bindings look like

DynBox and type registration allows to expose some information about what traits
certain types in Rust implement down to OCaml side, encoding those constraints
with polymorphic variants:

```ocaml
module Animal = struct
  type nonrec t =
    [ `Ocaml_rs_smartptr_test_stubs_animal_proxy | `Core_marker_send ]
      Ocaml_rs_smartptr.Rusty_obj.t

  external name : t -> string = "animal_name"
  external noise : t -> string = "animal_noise"
  external talk : t -> unit = "animal_talk"
end

module Sheep = struct
  type nonrec t =
    [ `Ocaml_rs_smartptr_test_stubs_sheep
    | `Core_marker_sync
    | `Core_marker_send
    | `Ocaml_rs_smartptr_test_stubs_animal_proxy
    ]
      Ocaml_rs_smartptr.Rusty_obj.t

  external create : string -> t = "sheep_create"
  external is_naked : t -> bool = "sheep_is_naked"
  external sheer : t -> unit = "sheep_sheer"
end

module Wolf = struct
  type nonrec t =
    [ `Ocaml_rs_smartptr_test_stubs_wolf
    | `Core_marker_sync
    | `Core_marker_send
    | `Ocaml_rs_smartptr_test_stubs_animal_proxy
    ]
      Ocaml_rs_smartptr.Rusty_obj.t

  external create : string -> t = "wolf_create"
  external set_hungry : t -> bool -> unit = "wolf_set_hungry"
end

module Test_callback = struct
  external call_cb : Wolf.t -> (Wolf.t -> Animal.t) -> Animal.t = "call_cb"
end
```

### Using the generated OCaml bindings

Using the binginds in OCaml is pretty straightforward:

```ocaml
open Stubs

let sheep_test () =
  print_endline "\n*** Sheep test";
  let sheep = Sheep.create "dolly" in
  Animal.talk (sheep :> Animal.t);
  Sheep.sheer sheep;
  Animal.talk (sheep :> Animal.t)
;;

let wolf_test () =
  print_endline "\n*** Wolf test";
  let wolf = Wolf.create "big bad wolf" in
  Animal.talk (wolf :> Animal.t);
  let animal =
    Test_callback.call_cb wolf (fun wolf ->
      print_endline "(wolf gets modified inside a callback!)";
      Wolf.set_hungry wolf true;
      (wolf :> Animal.t))
  in
  Animal.talk animal
;;

let main () =
  sheep_test ();
  wolf_test ()
;;

let () = main ()
```

Important note: when your project relies on DynBox type registry, it is
important to depend on `ocaml-rs-smartptr` OCaml library:

```
(library
 (name test_lib)
 (libraries ocaml-rs-smartptr))
```

 Type registration is also decentralized and is based on `inventory` crate, so
 during program initialization, all type conversions need to be registered. This
 is done in `ocaml-rs-smartptr` OCaml library, which is linked with `-linkall`
 flag, so initialization code will run whenever you link to this library.

## Test Project

A test project is included to demonstrate the usage of the `ocaml-rs-smartptr`
library. It includes examples of creating and manipulating Rust objects in
OCaml, coercing `Sheep` and `Wolf` to `Animal`, and interacting with these
objects through the `Animal` trait methods.

You can find it in [test subdirectory](./test)
