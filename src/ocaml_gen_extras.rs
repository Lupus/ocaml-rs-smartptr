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

#[derive(From, Deref, DerefMut, AsRef, AsMut)]
pub struct WithTypeParam<const C: char, T: ocaml::FromValue + ocaml::ToValue>(T);

impl<const C: char, T: ocaml::FromValue + ocaml::ToValue + OCamlDesc>
    WithTypeParam<C, T>
{
    pub fn new(v: T) -> Self {
        Self(v)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<const C: char, T: ocaml::FromValue + ocaml::ToValue + OCamlDesc> OCamlDesc
    for WithTypeParam<C, T>
{
    fn ocaml_desc(env: &ocaml_gen::Env, generics: &[&str]) -> String {
        format!("('{} {})", C, T::ocaml_desc(env, generics))
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

impl<const C: char, T: ocaml::FromValue + ocaml::ToValue + OCamlBinding + OCamlDesc>
    OCamlBinding for WithTypeParam<C, T>
{
    fn ocaml_binding(
        env: &mut ::ocaml_gen::Env,
        rename: Option<&'static str>,
        new_type: bool,
    ) -> String {
        let ty_id = Self::unique_id();

        if new_type {
            let orig = T::ocaml_binding(env, rename, new_type);
            insert_type_params(&orig, format!("'{}", C).as_str()).unwrap()
        } else {
            let name = Self::ocaml_desc(env, &[]);
            let ty_name = rename.expect("bug in ocaml-gen: rename should be Some");
            env.add_alias(ty_id, ty_name);

            format!("type nonrec '{} {} = '{} {}", C, ty_name, C, name)
        }
    }
}

unsafe impl<const C: char, T: ocaml::FromValue + ocaml::ToValue> ocaml::ToValue
    for WithTypeParam<C, T>
{
    fn to_value(&self, gc: &ocaml::Runtime) -> ocaml::Value {
        self.0.to_value(gc)
    }
}

unsafe impl<const C: char, T: ocaml::FromValue + ocaml::ToValue> ocaml::FromValue
    for WithTypeParam<C, T>
{
    fn from_value(v: ocaml::Value) -> Self {
        Self(T::from_value(v))
    }
}

impl<T, const C: char> From<T> for WithTypeParam<C, DynBox<T>>
where
    T: Send + 'static,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}
