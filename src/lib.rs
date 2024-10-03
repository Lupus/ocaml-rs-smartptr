pub mod callable;
pub mod func;
pub mod ml_box;
pub mod ptr;
pub mod registry;
mod type_name;
pub mod util;
pub mod stubs;

pub use ocaml_rs_smartptr_macro::register_trait;
pub use ocaml_rs_smartptr_macro::register_type;

pub use inventory;

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
