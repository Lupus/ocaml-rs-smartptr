use ocaml_gen::OCamlDesc; // Importing OCamlDesc trait for describing OCaml types

use crate::callable::Callable;
use crate::ml_box::MlBox;
use std::marker::PhantomData;
use std::panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe};

/// OCamlFunc is a wrapper around MlBox that represents an OCaml function.                                                                                        
/// It holds a reference to the OCaml function and ensures that it is safe to call                                                                                
/// from Rust. The PhantomData is used to keep track of the argument and return types.
#[derive(Debug)]
pub struct OCamlFunc<Args, Ret>(MlBox, AssertUnwindSafe<PhantomData<(Args, Ret)>>);

// As OCamlFunc is a wraper on top of MlBox, we mark OCamlFunc as Send + Sync as
// MlBox itself
unsafe impl<Args, Ret> Send for OCamlFunc<Args, Ret> {}
unsafe impl<Args, Ret> Sync for OCamlFunc<Args, Ret> {}

assert_impl_all!(OCamlFunc<(ocaml::Value,),ocaml::Value>: Send, Sync, UnwindSafe, RefUnwindSafe);

impl<Args, Ret> OCamlFunc<Args, Ret> {
    /// Creates a new OCamlFunc from an OCaml value.                                                                                                              
    /// This function takes an OCaml runtime handle to ensure that the operation                                                                                  
    /// is called while the OCaml domain lock is acquired.
    pub fn new(gc: &ocaml::Runtime, v: ocaml::Value) -> Self {
        OCamlFunc(MlBox::new(gc, v), AssertUnwindSafe(PhantomData))
    }
}

impl<Args, Ret> Clone for OCamlFunc<Args, Ret> {
    /// Clones the OCamlFunc, creating a new instance with the same underlying OCaml function.
    /// Custom Clone implementation lifts the requirements for Args and Ret to be Clone
    fn clone(&self) -> Self {
        OCamlFunc(self.0.clone(), AssertUnwindSafe(PhantomData))
    }
}

unsafe impl<Args, Ret> ocaml::FromValue for OCamlFunc<Args, Ret> {
    /// Converts an OCaml value to an OCamlFunc.                                                                                                                  
    /// This function should ideally receive a runtime handle, but it assumes that                                                                                
    /// it is not called manually on a non-OCaml thread.
    fn from_value(v: ocaml::Value) -> Self {
        OCamlFunc::new(unsafe { ocaml::Runtime::recover_handle() }, v)
    }
}

impl<Args: Callable<Ret>, Ret: ocaml::FromValue> OCamlFunc<Args, Ret>
where
    Ret: OCamlDesc,
{
    /// Calls the OCaml function with the provided arguments.                                                                                                     
    /// This function ensures that the OCaml runtime is properly handled.
    pub fn call(&self, gc: &ocaml::Runtime, args: Args) -> Ret {
        args.call_with(gc, self.0.as_value(gc))
    }
}

/// OCamlDesc impl for OCamlFunc is a thin wrapper on top of corresponding
/// methods in Callable.
impl<Args, Ret> OCamlDesc for OCamlFunc<Args, Ret>
where
    Args: Callable<Ret>,
    Ret: ocaml::FromValue + OCamlDesc,
{
    /// Generates the OCaml type description for the function.
    fn ocaml_desc(env: &::ocaml_gen::Env, generics: &[&str]) -> String {
        Args::ocaml_desc(env, generics)
    }

    /// Generates a unique ID for the function.
    fn unique_id() -> u128 {
        Args::unique_id()
    }
}
