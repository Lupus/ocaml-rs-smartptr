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
