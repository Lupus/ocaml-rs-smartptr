use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use derive_more::{
    derive::{AsMut, AsRef, Deref, DerefMut},
    From, Into,
};

use ocaml_gen::{OCamlBinding, OCamlDesc};

use crate::ptr::DynBox;

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

use std::marker::PhantomData;

pub trait TypeParams {
    fn params_string() -> String;
    fn params_count() -> usize;
}

pub struct P1<const C: char>;

impl<const C: char> TypeParams for P1<C> {
    fn params_string() -> String {
        format!("'{}", C)
    }
    fn params_count() -> usize {
        1
    }
}

pub struct P2<const C1: char, const C2: char>;

impl<const C1: char, const C2: char> TypeParams for P2<C1, C2> {
    fn params_string() -> String {
        format!("('{}, '{})", C1, C2)
    }
    fn params_count() -> usize {
        2
    }
}

pub struct P3<const C1: char, const C2: char, const C3: char>;

impl<const C1: char, const C2: char, const C3: char> TypeParams for P3<C1, C2, C3> {
    fn params_string() -> String {
        format!("('{}, '{}, '{})", C1, C2, C3)
    }
    fn params_count() -> usize {
        3
    }
}

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
    pub fn new(v: T) -> Self {
        Self(v, PhantomData)
    }

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
    fn ocaml_binding(
        env: &mut ::ocaml_gen::Env,
        rename: Option<&'static str>,
        new_type: bool,
    ) -> String {
        let ty_id = Self::unique_id();

        if new_type {
            let orig = T::ocaml_binding(env, rename, new_type);
            insert_type_params(&orig, &P::params_string()).unwrap()
        } else {
            let name = Self::ocaml_desc(env, &[]);
            let ty_name = rename.expect("bug in ocaml-gen: rename should be Some");
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

impl<T, P: TypeParams> From<T> for WithTypeParams<P, DynBox<T>>
where
    T: Send + 'static,
{
    fn from(value: T) -> Self {
        Self(value.into(), PhantomData)
    }
}

pub struct OcamlGenPlugin {
    generator: fn(&mut ocaml_gen::Env) -> String,
    crate_name: &'static str,
}

impl OcamlGenPlugin {
    pub const fn new(
        crate_name: &'static str,
        generator: fn(&mut ocaml_gen::Env) -> String,
    ) -> Self {
        OcamlGenPlugin {
            crate_name,
            generator,
        }
    }

    fn generate(&self, env: &mut ocaml_gen::Env) -> String {
        (self.generator)(env)
    }

    fn crate_name(&self) -> &'static str {
        self.crate_name
    }
}

inventory::collect!(OcamlGenPlugin);

pub fn stubs_gen_main() -> std::io::Result<()> {
    crate::registry::initialize_plugins();
    let env = &mut ocaml_gen::Env::new();
    let args: Vec<String> = env::args().skip(1).collect();

    println!("Detected OcamlGen Plugins:");
    for plugin in inventory::iter::<OcamlGenPlugin> {
        let crate_name = plugin.crate_name();
        if args.is_empty() || args.contains(&crate_name.to_string()) {
            let w = plugin.generate(env);

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
