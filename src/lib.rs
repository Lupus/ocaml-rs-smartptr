pub mod callable;
pub mod func;
pub mod ml_box;
pub mod ocaml_gen_extras;
pub mod ptr;
pub mod registry;
pub mod stubs;
mod type_name;
pub mod util;

pub use ocaml_rs_smartptr_macro::register_trait;
pub use ocaml_rs_smartptr_macro::register_type;

pub use inventory;

#[macro_use]
extern crate static_assertions;

#[macro_export]
macro_rules! register_rtti {
    ($($code:tt)*) => {
        $crate::inventory::submit! {
            $crate::registry::Plugin::new(|| {
                $($code)*
            })
        }
    };
}

#[macro_export]
macro_rules! ocaml_gen_bindings {
    ($($code:tt)*) => {
        $crate::inventory::submit! {
            $crate::ocaml_gen_extras::OcamlGenPlugin::new(std::env!("CARGO_PKG_NAME"),|ocaml_gen_env: &mut ocaml_gen::Env| {
                use std::fmt::Write;
                let mut w = String::new();

                #[allow(unused_macros)]
                macro_rules! decl_module {
                    ($name:expr, $content:tt) => {
                        ocaml_gen::decl_module!(w, ocaml_gen_env, $name, $content);
                    };
                }

                #[allow(unused_macros)]
                macro_rules! decl_type {
                    ($type:ty => $name:expr) => {
                        ocaml_gen::decl_type!(w, ocaml_gen_env, $type => $name);
                    };
                }

                #[allow(unused_macros)]
                macro_rules! decl_func {
                    ($func:ident => $name:expr) => {
                        ocaml_gen::decl_func!(w, ocaml_gen_env, $func => $name);
                    };
                }

                #[allow(unused_macros)]
                macro_rules! decl_type_alias {
                    ($new:expr => $ty:ty) => {
                        ocaml_gen::decl_type_alias!(w, ocaml_gen_env, $new => $ty);
                    };
                }

                #[allow(unused_macros)]
                macro_rules! decl_fake_generic {
                    ($name:ident, $i:expr) => {
                        ocaml_gen::decl_fake_generic!(w, ocaml_gen_env, $name, $i);
                    };
                }

                {
                    $($code)*
                }

                w
            })
        }
    };
}
