use ocaml_gen::OCamlDesc;

use crate::callable::Callable;
use crate::ml_box::MlBox;
use std::marker::PhantomData;
use std::panic::UnwindSafe;

#[derive(Debug, Clone)]
pub struct OCamlFunc<Args, Ret>(MlBox, PhantomData<(Args, Ret)>);

unsafe impl<Args, Ret> Send for OCamlFunc<Args, Ret> {}
impl<Args, Ret> UnwindSafe for OCamlFunc<Args, Ret> {}

impl<Args, Ret> OCamlFunc<Args, Ret> {
    pub fn new(gc: &ocaml::Runtime, v: ocaml::Value) -> Self {
        OCamlFunc(MlBox::new(gc, v), PhantomData)
    }
}

unsafe impl<Args, Ret> ocaml::FromValue for OCamlFunc<Args, Ret> {
    fn from_value(v: ocaml::Value) -> Self {
        /* from_value should really receive runtime handle :shrug: */
        /* let's just assume that no one is going to call from_value manually on
         * a weird thread... */
        OCamlFunc::new(unsafe { ocaml::Runtime::recover_handle() }, v)
    }
}

impl<Args: Callable<Ret>, Ret: ocaml::FromValue> OCamlFunc<Args, Ret>
where
    Ret: OCamlDesc,
{
    pub fn call(&self, gc: &ocaml::Runtime, args: Args) -> Ret {
        args.call_with(gc, self.0.as_value(gc))
    }
}

impl<Args, Ret> OCamlDesc for OCamlFunc<Args, Ret>
where
    Args: Callable<Ret>,
    Ret: ocaml::FromValue + OCamlDesc,
{
    fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
        Args::ocaml_desc(env, generics)
    }

    fn unique_id() -> u128 {
        Args::unique_id()
    }
}
