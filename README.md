# Smart Pointer for ocaml-rs

**WARNING**: Highly experimental code, do not use in production!

## Overview

This project provides a smart pointer implementation for `ocaml-rs`, allowing for safe interaction between Rust and OCaml. The library includes various utilities and extensions for generating OCaml bindings, handling OCaml functions, and managing OCaml values in Rust.

## Features

- **OCaml Function Wrappers**: Safe wrappers around OCaml functions.
- **Type Registration**: Mechanisms to register Rust types and traits for use in OCaml.
- **Callable Trait**: A trait representing functions or closures that can be called with a set of arguments to produce a return value.
- **Type Name Utilities**: Helpers for extracting and converting type names.

## Modules

### `src/func.rs`

- **OCamlFunc**: A wrapper around `MlBox` representing an OCaml function. It ensures safe calls from Rust.
- **OCamlDesc Implementation**: Provides OCaml type descriptions for functions.

### `src/type_name.rs`

- **extract_type_name**: Extracts the core type name from a type string.
- **capture_segments**: Captures segments of a type string until the core type.
- **convert_to_snake_case**: Converts a module path to snake_case.
- **capitalize_first_letter**: Capitalizes the first letter of a string.
- **get_type_name**: Returns the core type name.
- **snake_case_of_fully_qualified_name**: Converts a fully qualified name to snake_case with the first letter capitalized.

### `src/callable.rs`

- **Callable Trait**: Represents a function or closure that can be called with a set of arguments to produce a return value.
- **Macro Implementations**: Macros to generate the `call_with` function for tuples of different sizes.

### `src/ocaml_gen_extras.rs`

- **PolymorphicValue**: A wrapper around `ocaml::Value` printed as an OCaml polymorphic type.
- **TypeParams Trait**: Represents type parameters for generic types.
- **WithTypeParams**: A thin wrapper around a type with type parameters.
- **OcamlGenPlugin**: Represents a plugin for generating OCaml bindings.

## Usage

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

### Generating OCaml Bindings

Use the `ocaml_gen_bindings` macro to generate OCaml bindings:

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

## Test Project

A test project is included to demonstrate the usage of the `ocaml-rs-smartptr` library. It includes examples of creating and manipulating Rust objects in OCaml, coercing `Sheep` and `Wolf` to `Animal`, and interacting with these objects through the `Animal` trait methods.

### Structure

- **Rust Code**:
  - `src/animals.rs`: Defines the `Animal` trait and its implementations for `Sheep` and `Wolf`.
  - `src/stubs.rs`: Provides stubs for OCaml (`extern "C"` functions) and registers the types and traits.

- **OCaml Code**:
  - `test/Stubs.ml`: Defines OCaml modules and external functions for `Animal`, `Sheep`, and `Wolf`.
  - `test/test.ml`: Contains test cases for `Sheep` and `Wolf`, demonstrating their creation, coercion to `Animal`, and interaction through the `Animal` trait methods.

## License

This project is licensed under the MIT License.
