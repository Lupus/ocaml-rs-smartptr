#![doc = "This module provides additional utilities and extensions for generating OCaml bindings."]

use std::env;
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;

use derive_more::{
    derive::{AsMut, AsRef, Deref, DerefMut},
    From, Into,
};

use ocaml_gen::{OCamlBinding, OCamlDesc};

use crate::ptr::DynBox;

/// A wrapper around `ocaml::Value` that is printed by `ocaml_gen` as an OCaml
/// polymorphic type, i.e., `'a` or `'b`, where the `a` or `b` symbol comes from
/// the const `C: char` of this `PolymorphicValue`.
#[derive(From, Into, Deref, DerefMut)]
pub struct PolymorphicValue<const C: char>(ocaml::Value);

impl<const C: char> ocaml_gen::OCamlDesc for PolymorphicValue<C> {
    fn ocaml_desc(_env: &ocaml_gen::Env, _generics: &[&str]) -> String {
        format!("'{}", C)
    }

    fn unique_id() -> u128 {
        panic!("unique_id is not supported for PolymorphicValue")
    }
}

unsafe impl<const C: char> ocaml::ToValue for PolymorphicValue<C> {
    fn to_value(&self, _gc: &ocaml::Runtime) -> ocaml::Value {
        self.0.clone()
    }
}

unsafe impl<const C: char> ocaml::FromValue for PolymorphicValue<C> {
    fn from_value(v: ocaml::Value) -> Self {
        Self(v)
    }
}

/// A trait that is implemented by `P1`, `P2`, etc., used as a link between
/// concrete `P1`, `P2`, etc., and the `WithTypeParams` wrapper type below.
pub trait TypeParams {
    /// Returns a string representation of the type parameters.
    fn params_string() -> String;
    /// Returns the count of type parameters.
    fn params_count() -> usize;
}

/// P1 is for a single type parameter 'x where x is const C: char
pub struct P1<const C: char>;

/// Implementation of `TypeParams` for a single type parameter `'x` where `x` is
/// `const C: char`.
impl<const C: char> TypeParams for P1<C> {
    fn params_string() -> String {
        format!("'{}", C)
    }
    fn params_count() -> usize {
        1
    }
}

/// P2 is for a two type parameters 'x,'y where x is const C1: char and y is
/// const C2: char
pub struct P2<const C1: char, const C2: char>;

/// Implementation of `TypeParams` for two type parameters `'x, 'y` where `x` is
/// `const C1: char` and `y` is `const C2: char`.
impl<const C1: char, const C2: char> TypeParams for P2<C1, C2> {
    fn params_string() -> String {
        format!("('{}, '{})", C1, C2)
    }
    fn params_count() -> usize {
        2
    }
}

/// Same as P2 but for three type parameters
pub struct P3<const C1: char, const C2: char, const C3: char>;

/// Implementation of `TypeParams` for three type parameters `'x, 'y, 'z` where
/// `x` is `const C1: char`, `y` is `const C2: char`, and `z` is `const C3:
/// char`.
impl<const C1: char, const C2: char, const C3: char> TypeParams for P3<C1, C2, C3> {
    fn params_string() -> String {
        format!("('{}, '{}, '{})", C1, C2, C3)
    }
    fn params_count() -> usize {
        3
    }
}

/// Thin wrapper around T which adds ability to print T into ocaml_desc as a
/// type with type parameters
#[derive(From, Deref, DerefMut, AsRef, AsMut)]
pub struct WithTypeParams<P: TypeParams, T: ocaml::FromValue + ocaml::ToValue>(
    #[deref]
    #[deref_mut]
    #[as_ref]
    #[as_mut]
    T,
    PhantomData<P>,
);

impl<P: TypeParams, T: ocaml::FromValue + ocaml::ToValue + OCamlDesc>
    WithTypeParams<P, T>
{
    /// Creates a new `WithTypeParams` instance.
    pub fn new(v: T) -> Self {
        Self(v, PhantomData)
    }

    /// Consumes the `WithTypeParams` instance and returns the inner value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<P: TypeParams, T: ocaml::FromValue + ocaml::ToValue + OCamlDesc> OCamlDesc
    for WithTypeParams<P, T>
{
    fn ocaml_desc(env: &ocaml_gen::Env, generics: &[&str]) -> String {
        format!("({} {})", P::params_string(), T::ocaml_desc(env, generics))
    }

    fn unique_id() -> u128 {
        T::unique_id()
    }
}

fn insert_type_params(
    input_string: &str,
    type_params: &str,
) -> Result<String, &'static str> {
    let type_nonrec = "type nonrec ";

    if let Some(type_index) = input_string.find(type_nonrec) {
        let insert_index = type_index + type_nonrec.len();
        let mut result = String::from(&input_string[..insert_index]);
        result.push_str(type_params);
        result.push(' ');
        result.push_str(&input_string[insert_index..]);
        Ok(result)
    } else {
        Err("Could not find 'type nonrec' in the input string")
    }
}

impl<P: TypeParams, T: ocaml::FromValue + ocaml::ToValue + OCamlBinding + OCamlDesc>
    OCamlBinding for WithTypeParams<P, T>
{
    /// Generates the OCaml binding for the type with type parameters.
    fn ocaml_binding(
        env: &mut ::ocaml_gen::Env,
        rename: Option<&'static str>,
        new_type: bool,
    ) -> String {
        let ty_id = Self::unique_id();

        if new_type {
            let orig = T::ocaml_binding(env, rename, new_type);
            // Unfortunately, `OCamlBinding` is not very friendly to composing the
            // bindings, so we have to parse the generated binding and adjust it.
            insert_type_params(&orig, &P::params_string()).unwrap()
        } else {
            let name = Self::ocaml_desc(env, &[]);
            let ty_name = rename.expect("bug in `ocaml_gen`: rename should be `Some`");
            env.add_alias(ty_id, ty_name);

            format!(
                "type nonrec {} {} = {} {}",
                P::params_string(),
                ty_name,
                P::params_string(),
                name
            )
        }
    }
}

unsafe impl<P: TypeParams, T: ocaml::FromValue + ocaml::ToValue> ocaml::ToValue
    for WithTypeParams<P, T>
{
    fn to_value(&self, gc: &ocaml::Runtime) -> ocaml::Value {
        self.0.to_value(gc)
    }
}

unsafe impl<P: TypeParams, T: ocaml::FromValue + ocaml::ToValue> ocaml::FromValue
    for WithTypeParams<P, T>
{
    fn from_value(v: ocaml::Value) -> Self {
        Self(T::from_value(v), PhantomData)
    }
}

/// This allows `.into()` from right to `TypeParams<P, DynBox<T>>`
impl<T, P: TypeParams> From<T> for WithTypeParams<P, DynBox<T>>
where
    T: Send + 'static,
{
    fn from(value: T) -> Self {
        Self(value.into(), PhantomData)
    }
}

#[macro_export]
macro_rules! ocaml_export {
    ($inner_type:ty, $new_type:ident, $ocaml_path:expr) => {
        #[allow(dead_code)]
        pub struct $new_type($inner_type);

        impl ::std::ops::Deref for $new_type {
            type Target = $inner_type;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ::std::ops::DerefMut for $new_type {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl ::std::convert::From<$inner_type> for $new_type {
            fn from(inner: $inner_type) -> Self {
                Self(inner)
            }
        }

        unsafe impl ocaml::ToValue for $new_type {
            fn to_value(&self, rt: &ocaml::Runtime) -> ocaml::Value {
                self.0.to_value(rt)
            }
        }

        unsafe impl ocaml::FromValue for $new_type {
            fn from_value(v: ocaml::Value) -> Self {
                $new_type::from_value(v)
            }
        }

        impl ::ocaml_gen::OCamlDesc for $new_type {
            fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
                // We clone an env
                let mut env = env.clone();
                // Ask our inner type to produce ocaml binding for a new type in
                // the cloned env under desired name, we ignore the actial
                // binding code returned by `ocaml_binding` as we don't need it
                <$inner_type as ::ocaml_gen::OCamlBinding>::ocaml_binding(
                    &mut env,
                    Some($ocaml_path),
                    true,
                );
                // Call ocaml_desc for our inner type in this new env with
                // defined binding
                let res =
                    <$inner_type as ::ocaml_gen::OCamlDesc>::ocaml_desc(&env, generics);
                // Discard the env to avoid panics on drop as we're still nested
                // in some module etc
                env.discard();
                // Return the ocaml_desc produced in fake env
                res
            }

            fn unique_id() -> u128 {
                <$inner_type as ::ocaml_gen::OCamlDesc>::unique_id()
            }
        }

        impl ::ocaml_gen::OCamlBinding for $new_type {
            fn ocaml_binding(
                env: &mut ::ocaml_gen::Env,
                rename: Option<&'static str>,
                new_type: bool,
            ) -> String {
                let ty_id = <Self as ::ocaml_gen::OCamlDesc>::unique_id();
                let name = <Self as ::ocaml_gen::OCamlDesc>::ocaml_desc(env, &[]);

                if new_type {
                    panic!("can't declare a new type for {}, as it's exported from other lib, \
                        you can declare an alias for it if you really want to", stringify!($new_type));
                } else {
                    let ty_name =
                        rename.expect("bug in ocaml-gen: rename should be Some");
                    env.add_alias(ty_id, ty_name);
                    format!("type nonrec {} = {}", ty_name, name)
                }
            }
        }
    };
}

/// Represents a plugin for generating OCaml bindings.
/// It contains a generator function and the name of the crate.
pub struct OcamlGenPlugin {
    /// The function that generates the OCaml bindings.
    generator: fn(&mut ocaml_gen::Env) -> String,
    /// Name of the crate where this plugin was registered
    crate_name: &'static str,
}

impl OcamlGenPlugin {
    /// Creates a new `OcamlGenPlugin` instance.
    pub const fn new(
        crate_name: &'static str,
        generator: fn(&mut ocaml_gen::Env) -> String,
    ) -> Self {
        OcamlGenPlugin {
            crate_name,
            generator,
        }
    }

    /// Generates the OCaml bindings using the provided environment.
    fn generate(&self, env: &mut ocaml_gen::Env) -> String {
        (self.generator)(env)
    }

    /// Returns the name of the crate associated with this plugin.
    fn crate_name(&self) -> &'static str {
        self.crate_name
    }
}

inventory::collect!(OcamlGenPlugin);

/// Main function for stubs generation binaries. It collects `OcamlGenPlugin`s
/// registered in other libraries and writes one `.ml` file per crate with
/// generated OCaml bindings.
pub fn stubs_gen_main() -> std::io::Result<()> {
    crate::registry::initialize_plugins();
    let args: Vec<String> = env::args().skip(1).collect();

    println!("Detected OcamlGen Plugins:");
    for plugin in inventory::iter::<OcamlGenPlugin> {
        let crate_name = plugin.crate_name();
        if args.is_empty() || args.contains(&crate_name.to_string()) {
            let w = std::panic::catch_unwind(|| {
                let env = &mut ocaml_gen::Env::new();
                plugin.generate(env)
            })
            .map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("plugin from crate `{}' failed: {:?}", crate_name, err),
                )
            })?;

            let file_name = format!(
                "{}.ml",
                crate_name
                    .replace('-', "_")
                    .chars()
                    .enumerate()
                    .map(|(i, c)| if i == 0 {
                        c.to_uppercase().next().unwrap()
                    } else {
                        c
                    })
                    .collect::<String>()
            );

            let path = Path::new(&file_name);
            let mut file = File::create(path)?;
            file.write_all(w.as_bytes())?;
            println!(" - Crate: {}, generated: {}", crate_name, file_name);
        }
    }

    Ok(())
}
