use ocaml_gen::{OCamlBinding, OCamlDesc};
use static_assertions::{assert_impl_all, assert_not_impl_all};
use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};

use crate::{registry, type_name};

pub struct DynBox<T>
where
    T: Send + ?Sized,
{
    inner: Arc<dyn Any + Sync + Send>,
    _phantom: PhantomData<fn(T) -> T>, // https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns
}

impl<T: 'static + Send> DynBox<T> {
    // Function to create a DynBox with Mutex
    pub fn new_exclusive(value: T) -> Self {
        registry::register_type::<T>();
        registry::register_type::<Arc<T>>();
        DynBox {
            inner: Arc::new(Mutex::new(value)),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static + Sync + Send> DynBox<T> {
    // Function to create a DynBox with RwLock
    pub fn new_shared(value: T) -> Self {
        registry::register_type::<T>();
        registry::register_type::<Arc<T>>();
        DynBox {
            inner: Arc::new(RwLock::new(value)),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static + Send + ?Sized> DynBox<T> {
    fn into_raw(self) -> *const (dyn Any + Send + Sync) {
        Arc::into_raw(self.inner)
    }

    fn from_raw(ptr: *const (dyn Any + Send + Sync)) -> Self {
        DynBox {
            inner: unsafe { Arc::from_raw(ptr) },
            _phantom: PhantomData,
        }
    }

    pub fn coerce(&self) -> registry::Handle<T> {
        registry::coerce::<T>(self.inner.clone())
    }

    pub fn coerce_mut(&self) -> registry::HandleMut<T> {
        registry::coerce_mut::<T>(self.inner.clone())
    }
}

impl<T: 'static + Send + ?Sized> Clone for DynBox<T> {
    fn clone(&self) -> Self {
        DynBox {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

// Function to get u128 hash of a TypeId
fn type_id_hash_u128<T: ?Sized + 'static>() -> u128 {
    let type_id = TypeId::of::<T>();
    let mut hasher = std::hash::DefaultHasher::new();
    type_id.hash(&mut hasher);
    let hash64 = hasher.finish();

    // Combine two 64-bit parts to make a u128

    ((hash64 as u128) << 64) | (hash64 as u128)
}

impl<T: ?Sized + Send + 'static> OCamlDesc for DynBox<T> {
    fn ocaml_desc(env: &::ocaml_gen::Env, _generics: &[&str]) -> String {
        let type_id = <Self as OCamlDesc>::unique_id();
        env.get_type(type_id, type_name::get_type_name::<T>().as_str())
            .0
    }

    fn unique_id() -> u128 {
        type_id_hash_u128::<T>()
    }
}

impl<T: ?Sized + Send + 'static> OCamlBinding for DynBox<T> {
    fn ocaml_binding(
        env: &mut ::ocaml_gen::Env,
        rename: Option<&'static str>,
        new_type: bool,
    ) -> String {
        // register the new type
        let ty_id = Self::unique_id();

        if new_type {
            let name = Box::leak(Box::new(type_name::get_type_name::<T>()));
            let ty_name = rename.unwrap_or(name.as_str());
            env.new_type(ty_id, ty_name);
        }

        let name = Self::ocaml_desc(env, &[]);
        let names = registry::get_type_info::<T>().implementations;
        let variants = names
            .iter()
            .map(|type_str| type_name::snake_case_of_fully_qualified_name(type_str))
            .map(|v| "`".to_owned() + &v)
            .collect::<Vec<_>>()
            .join("|");

        if new_type {
            format!(
                "type nonrec {} = [ {} ] Ocaml_rs_smartptr.Rusty_obj.t",
                name, variants
            )
        } else {
            // add the alias
            let ty_name = rename.expect("bug in ocaml-gen: rename should be Some");
            env.add_alias(ty_id, ty_name);

            format!("type nonrec {} = {}", ty_name, name)
        }
    }
}

// Static assertions to verify that DynBox<T> is Sync and Send
assert_not_impl_all!(std::cell::RefCell<i32>: Sync); // RefCell<i32> is not Sync
assert_impl_all!(DynBox<std::cell::RefCell<i32>>: Sync, Send); // But DynBox allows RefCell<i32>
assert_impl_all!(DynBox<i32>: Sync, Send); // And DynBox allows Sync + Send obviously

struct RustyObj(*const (dyn Any + Send + Sync));

unsafe extern "C" fn rusty_obj_finalizer(v: ocaml::Raw) {
    let ptr = v.as_pointer::<RustyObj>();
    // Actual type parameter T for DynBox<T> is irrelevant here, dyn Any inside
    // DynBox would know which destructor to call, and T is only for PhantomData
    let dynbox: DynBox<i32> = DynBox::from_raw(ptr.as_ref().0);
    drop(dynbox);
    ptr.drop_in_place();
}

impl ocaml::Custom for RustyObj {
    const NAME: &'static str = "RustyObj\0";

    const OPS: ocaml::custom::CustomOps = ocaml::custom::CustomOps {
        identifier: Self::NAME.as_ptr() as *mut ocaml::sys::Char,
        finalize: Some(rusty_obj_finalizer),
        ..ocaml::custom::DEFAULT_CUSTOM_OPS
    };
}

unsafe impl<T> ocaml::FromValue for DynBox<T>
where
    T: Send + ?Sized + 'static,
{
    fn from_value(v: ocaml::Value) -> Self {
        let ptr = unsafe { v.raw().as_pointer::<RustyObj>() };
        let orig_dynbox = DynBox::from_raw(ptr.as_ref().0);
        let dynbox = orig_dynbox.clone();
        let _ = std::mem::ManuallyDrop::new(orig_dynbox);
        dynbox
    }
}

unsafe impl<T> ocaml::ToValue for DynBox<T>
where
    T: Send + ?Sized + 'static,
{
    fn to_value(&self, rt: &ocaml::Runtime) -> ocaml::Value {
        let ptr = DynBox::into_raw(self.clone());
        let rusty_obj = RustyObj(ptr);
        ocaml::Pointer::from(rusty_obj).to_value(rt)
    }
}

impl<T> From<T> for DynBox<T>
where
    T: Send + 'static,
{
    fn from(value: T) -> Self {
        DynBox::new_exclusive(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as ocaml_rs_smartptr; // For proc macro use below to work
    use crate::register_type;
    use serial_test::serial;

    #[derive(Debug)]
    struct MyError {
        msg: String,
    }

    impl std::fmt::Display for MyError {
        fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            fmt.write_str(self.msg.as_str())
        }
    }

    impl std::error::Error for MyError {}

    fn get_error_message(error: DynBox<dyn std::error::Error + Send>) -> String {
        let error = error.coerce();
        error.to_string()
    }

    #[test]
    #[serial(registry)]
    fn test_bla() {
        register_type!({
            ty: crate::ptr::tests::MyError,
            marker_traits: [core::marker::Send],
            object_safe_traits: [std::error::Error],
        });
        let error = MyError {
            msg: String::from("bla-bla-bla"),
        };
        let orig_error_msg = error.to_string();
        let error = DynBox::new_shared(error);
        // The following line mimics the dynbox being sent to OCaml and received
        // back as another type
        let error = DynBox::from_raw(DynBox::into_raw(error));
        let wrapped_error_msg = get_error_message(error);
        assert_eq!(wrapped_error_msg, orig_error_msg);
    }
}
