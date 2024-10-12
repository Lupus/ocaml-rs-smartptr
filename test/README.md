# Test Project using `ocaml-rs-smartptr` Library

This test project demonstrates how to use the `ocaml-rs-smartptr` library to
create and manipulate Rust objects in OCaml. The project includes examples of
coercion in OCaml from `Sheep` and `Wolf` to `Animal`, which follows Rust
casting from corresponding objects to the object-safe trait `Animal`.

## Overview

The test case illustrates the following key points:
- Creation of `Sheep` and `Wolf` objects in OCaml.
- Coercion of `Sheep` and `Wolf` objects to the `Animal` trait.
- Interaction with these objects through the `Animal` trait methods.

## Structure of the Test Scenario

### 1. Rust Code

#### `src/animals.rs`
Defines the `Animal` trait and its implementations for `Sheep` and `Wolf`.

#### `src/stubs.rs`
Provides stubs for OCaml (`extern "C"` functions) and registers the types and traits.

### 2. OCaml Code

#### `test/Stubs.ml`

Defines OCaml modules and external functions for `Animal`, `Sheep`, and `Wolf`.
This module is generated with the help of `ocaml-gen` crate.

#### `test/test.ml`

Contains the test cases for `Sheep` and `Wolf`, demonstrating their creation,
coercion to `Animal`, and interaction through the `Animal` trait methods.
