use std::hash::Hash;

use highway::{HighwayHash, HighwayHasher}; // For hashing unique IDs
use ocaml_gen::{const_random, OCamlDesc};
use paste::paste; // For generating repetitive code

/// The `Callable` trait represents a function or closure that can be called
/// with a set of arguments to produce a return value. This trait is designed to
/// be used with OCaml values and provides methods for calling the function,
/// describing its arguments, and generating unique IDs for the function
/// signature.
///                                                                                                                                                                
/// # Type Parameters                                                                                                                                              
/// - `Ret`: The return type of the function, which must implement
///   `ocaml::FromValue` and `OCamlDesc`.
pub trait Callable<Ret>
where
    Ret: ocaml::FromValue + OCamlDesc,
{
    fn call_with(&self, gc: &ocaml::Runtime, func: ocaml::Value) -> Ret;
    /// Describes the arguments (i.e. calls OCamlDesc::ocaml_desc) of the
    /// function. This method should be provided by downstream trait
    /// implementations.
    fn describe_args(env: &::ocaml_gen::Env, generics: &[&str]) -> Vec<String>;
    /// Generates unique IDs for the function arguments. This method should be
    /// provided by downstream trait implementations.
    fn unique_id_args() -> Vec<u128>;
    /// ocaml_desc generates OCaml type signature for this Callable
    fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
        let args = Self::describe_args(env, generics)
            .into_iter()
            .map(|desc| format!("({})", desc))
            .collect::<Vec<_>>()
            .join(" -> ");
        format!("({} -> ({}))", args, Ret::ocaml_desc(env, generics))
    }

    fn unique_id() -> u128 {
        // Static randomized key for Callable
        let key = highway::Key([
            const_random!(u64),
            const_random!(u64),
            const_random!(u64),
            const_random!(u64),
        ]);
        // Hasher seeded with our key
        let mut hasher = HighwayHasher::new(key);
        // Hash all Callable arguments
        Self::unique_id_args()
            .iter()
            .for_each(|id| id.hash(&mut hasher));
        /* Hash return value */
        Ret::unique_id().hash(&mut hasher);
        let result = hasher.finalize128();
        (result[0] as u128) | ((result[1] as u128) << 64)
    }
    fn process_result(&self, res: Result<ocaml::Value, ocaml::Error>) -> Ret {
        let res = res.unwrap();
        Ret::from_value(res)
    }
}

impl<Ret: ocaml::FromValue + OCamlDesc> Callable<Ret> for () {
    fn call_with(&self, gc: &ocaml::Runtime, func: ocaml::Value) -> Ret {
        // We use .call1 with a single `()' argument as OCaml does not have a
        // notion of a function without arguments
        self.process_result(unsafe { func.call1(gc, ()) })
    }
    fn describe_args(env: &ocaml_gen::Env, generics: &[&str]) -> Vec<String> {
        // Just call OCamlDesc::ocaml_desc on `()' type
        vec![<() as OCamlDesc>::ocaml_desc(env, generics)]
    }
    fn unique_id_args() -> Vec<u128> {
        // Just call OCamlDesc::unique_id on `()' type
        vec![<() as OCamlDesc>::unique_id()]
    }
}

/// Macro to generate the `call_with` function for tuples of different sizes.
/// This macro handles special cases for tuples with 1, 2, and 3 elements by
/// generating the appropriate `func.call1`, `func.call2`, and `func.call3` calls.
/// For tuples with more than 3 elements, it generates a generic `func.call`
/// with the elements converted to OCaml values.
macro_rules! generate_call_with {
    ($idx:tt) => {
        fn call_with(&self, gc: &ocaml::Runtime, func: ocaml::Value) -> Ret {
            self.process_result(unsafe { func.call1(gc, &self.0) })
        }
    };
    ($idx1:tt, $idx2:tt) => {
        fn call_with(&self, gc: &ocaml::Runtime, func: ocaml::Value) -> Ret {
            self.process_result(unsafe { func.call2(gc, &self.0, &self.1) })
        }
    };
    ($idx1:tt, $idx2:tt, $idx3:tt) => {
        fn call_with(&self, gc: &ocaml::Runtime, func: ocaml::Value) -> Ret {
            self.process_result(unsafe { func.call3(gc, &self.0, &self.1, &self.2) })
        }
    };
    ($count:tt, $($idx:tt),*) => {
        fn call_with(&self, gc: &ocaml::Runtime, func: ocaml::Value) -> Ret {
            self.process_result(unsafe {
                func.call(
                    gc,
                    [
                        $(
                            self.$idx.to_value(gc),
                        )*
                    ],
                )
            })
        }
    };
}

/// Macro to implement the `Callable` trait for tuples of different sizes.
/// This macro uses the `generate_call_with` macro to generate the `call_with`
/// function and implements the `describe_args` and `unique_id_args` functions
/// for tuples of different sizes.
macro_rules! impl_callable_for_tuple {
    ($($idx:literal),+) => {
        paste! {
            impl<
                $(
                    [<T $idx>]: ocaml::ToValue + OCamlDesc,
                )*
                Ret: ocaml::FromValue + OCamlDesc,
            > Callable<Ret> for ($(
                [<T $idx>],
            )*)
            {
                generate_call_with! { $($idx),+ }
                fn describe_args(env: &::ocaml_gen::Env, generics: &[&str]) -> Vec<String> {
                    vec![
                        $(
                            [<T $idx>]::ocaml_desc(env, generics),
                        )*
                    ]
                }
                fn unique_id_args() -> Vec<u128> {
                    vec![
                        $(
                            [<T $idx>]::unique_id(),
                        )*
                    ]
                }
            }
        }
    };
}

// Implement the `Callable` trait for tuples of sizes 1 to 20.
impl_callable_for_tuple!(0);
impl_callable_for_tuple!(0, 1);
impl_callable_for_tuple!(0, 1, 2);
impl_callable_for_tuple!(0, 1, 2, 3);
impl_callable_for_tuple!(0, 1, 2, 3, 4);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16);
impl_callable_for_tuple!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17);
impl_callable_for_tuple!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18
);
impl_callable_for_tuple!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
);
impl_callable_for_tuple!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20
);
