#[ocaml::func]
pub fn ocaml_rs_smartptr_init_registry() {
    crate::registry::initialize_plugins()
}
