#![doc = r#"This module provides some stubs for OCaml (`extern "C"` functions). Not to be used from Rust."#]

#[ocaml::func]
pub fn ocaml_rs_smartptr_init_registry() {
    crate::registry::initialize_plugins()
}
