#![doc = r#"
This module provides the `DynBox` smart pointer, which is a wrapper around the
registry's `DynArc` with `PhantomData` for type safety. `DynBox` allows the user
to wrap the object in a `Mutex` or shared `RwLock`. By default, using `.into()`
will create a `Mutex`-protected version (exclusive).

## Key Components

- `DynBox<T>`: A smart pointer for dynamically typed Rust objects referenced
  from the OCaml side.
- `RustyObj`: A thin wrapper around a pointer to `DynArc`.

## Usage

### Creating a `DynBox`

You can create a `DynBox` with either a `Mutex` or `RwLock`:

```rust
use ocaml_rs_smartptr::ptr::DynBox;
let exclusive_box = DynBox::new_exclusive(42); // Mutex-protected
let shared_box = DynBox::new_shared("foo"); // RwLock-protected
```

### Coercion

The `coerce` and `coerce_mut` methods return a handle that holds a lock. Be
cautious to avoid deadlocks when using these methods.

### OCaml Integration

`DynBox` integrates with the `ocaml_gen` package by providing `OCamlDesc` and
`OCamlBinding` traits. Polymorphic variants are used to encode supported traits
of the Rust type that we wrap/bind.

Example:

```ocaml
module Animal = struct 
  type nonrec t = [ `Ocaml_rs_smartptr_test_stubs_animal_proxy|`Core_marker_send ] Ocaml_rs_smartptr.Rusty_obj.t
  external name : t -> string = "animal_name"
  external noise : t -> string = "animal_noise"
  external talk : t -> unit = "animal_talk"
end


module Sheep = struct 
  type nonrec t = [ `Ocaml_rs_smartptr_test_stubs_sheep|`Core_marker_sync|`Core_marker_send|`Ocaml_rs_smartptr_test_stubs_animal_proxy ] Ocaml_rs_smartptr.Rusty_obj.t
  external create : string -> t = "sheep_create"
  external is_naked : t -> bool = "sheep_is_naked"
  external sheer : t -> unit = "sheep_sheer"
end


module Wolf = struct 
  type nonrec t = [ `Ocaml_rs_smartptr_test_stubs_wolf|`Core_marker_sync|`Core_marker_send|`Ocaml_rs_smartptr_test_stubs_animal_proxy ] Ocaml_rs_smartptr.Rusty_obj.t
  external create : string -> t = "wolf_create"
  external set_hungry : t -> bool -> unit = "wolf_set_hungry"
end
```

This allows passing `Wolf` or `Sheep` whenever `Animal` is required by using
coercion operator in OCaml (`:>`).

### RustyObj

`RustyObj` is a thin wrapper around a pointer to `DynArc`. We convert `Arc` into
a raw pointer to hold that raw pointer in the OCaml heap, ensuring that moving
of that value by the OCaml GC does not affect any Rust invariants. Reverse
operation reconstructs the `Arc` from the raw pointer. This ensures that both
OCaml and Rust always hold valid Arc-baked references to objects they need.
"#]

use highway::{HighwayHash, HighwayHasher};
use ocaml_gen::{const_random, OCamlBinding, OCamlDesc};
use static_assertions::{assert_impl_all, assert_not_impl_all};
use std::any::{Any, TypeId};
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};

use crate::{registry, type_name};

/// A smart pointer around the registry's `DynArc` with `PhantomData` for type safety.
/// Allows the user to wrap the object in a `Mutex` or shared `RwLock`.
/// By default, using `.into()` will create a `Mutex`-protected version (exclusive).
pub struct DynBox<T>
where
    T: Send + ?Sized,
{
    inner: Arc<dyn Any + Sync + Send>,
    _phantom: PhantomData<fn(T) -> T>, // https://doc.rust-lang.org/nomicon/phantom-data.html#table-of-phantomdata-patterns
}

impl<T: 'static + Send> DynBox<T> {
    /// Creates a `DynBox` with a `Mutex`.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to be wrapped in the `DynBox`.
    ///
    /// # Returns
    ///
    /// A new `DynBox` instance with `Mutex` protection.
    pub fn new_exclusive(value: T) -> Self {
        registry::register_type::<T>();
        registry::register_type::<Arc<T>>();
        DynBox {
            inner: Arc::new(Mutex::new(value)),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static + Send + ?Sized> DynBox<T> {
    /// Creates a `DynBox` with a `Mutex` out of a Box'ed T. Useful if T is
    /// unsized, e.g. a `dyn Trait`.
    ///
    /// # Parameters
    ///
    /// - `value`: The value (inside a Box) to be wrapped in the `DynBox`.
    ///
    /// # Returns
    ///
    /// A new `DynBox` instance with `Mutex` protection.
    pub fn new_exclusive_boxed(value: Box<T>) -> Self {
        registry::register_type::<Box<T>>();
        registry::register_type::<Arc<Box<T>>>();
        DynBox {
            inner: Arc::new(Mutex::new(value)),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static + Sync + Send> DynBox<T> {
    /// Creates a `DynBox` with a `RwLock`.
    ///
    /// # Parameters
    ///
    /// - `value`: The value to be wrapped in the `DynBox`.
    ///
    /// # Returns
    ///
    /// A new `DynBox` instance with `RwLock` protection.
    pub fn new_shared(value: T) -> Self {
        registry::register_type::<T>();
        registry::register_type::<Arc<T>>();
        DynBox {
            inner: Arc::new(RwLock::new(value)),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static + Sync + Send + ?Sized> DynBox<T> {
    /// Creates a `DynBox` with a `RwLock` out of a Box'ed T. Useful if T is
    /// unsized, e.g. a `dyn Trait`.
    ///
    /// # Parameters
    ///
    /// - `value`: The value (inside a Box) to be wrapped in the `DynBox`.
    ///
    /// # Returns
    ///
    /// A new `DynBox` instance with `RwLock` protection.
    pub fn new_shared_boxed(value: Box<T>) -> Self {
        registry::register_type::<Box<T>>();
        registry::register_type::<Arc<Box<T>>>();
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

    /// Coerces the `DynBox` to a handle of the specified type.
    ///
    /// # Returns
    ///
    /// A handle to the coerced type. Note that this handle holds a lock, so use
    /// with care to avoid deadlocks.
    pub fn coerce(&self) -> registry::Handle<T> {
        registry::coerce::<T>(self.inner.clone())
    }

    /// Coerces the `DynBox` to a mutable handle of the specified type.
    ///
    /// # Returns
    ///
    /// A mutable handle to the coerced type. Note that this handle holds a
    /// lock, so use with care to avoid deadlocks.
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

impl<E> From<E> for DynBox<dyn std::error::Error + Send>
where
    E: std::error::Error + Send + 'static,
{
    fn from(err: E) -> Self {
        let boxed_err: Box<dyn std::error::Error + Send> = Box::new(err);
        DynBox::new_exclusive_boxed(boxed_err)
    }
}

impl<T: ?Sized + Send + 'static> OCamlDesc for DynBox<T> {
    fn ocaml_desc(env: &::ocaml_gen::Env, _generics: &[&str]) -> String {
        let type_id = <Self as OCamlDesc>::unique_id();
        let typ = env
            .get_type(type_id, type_name::get_type_name::<T>().as_str())
            .0;
        format!("_ {}'", typ)
    }

    fn unique_id() -> u128 {
        let key = highway::Key([
            const_random!(u64),
            const_random!(u64),
            const_random!(u64),
            const_random!(u64),
        ]);
        let mut hasher = HighwayHasher::new(key);
        let type_id = TypeId::of::<T>();
        type_id.hash(&mut hasher);
        let result = hasher.finalize128();
        (result[0] as u128) | ((result[1] as u128) << 64)
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
        let name = name
            .split_whitespace()
            .last()
            .expect("no last element :shrug:")
            .to_owned();
        let name = name
            .strip_suffix("'")
            .expect("dynbox type name does not end with `'`!");

        let names = registry::get_type_info::<T>().implementations;
        let variants = names
            .iter()
            .map(|type_str| type_name::snake_case_of_fully_qualified_name(type_str))
            .map(|v| "`".to_owned() + &v)
            .collect::<Vec<_>>()
            .join("|");

        if new_type {
            format!(
                "type tags = [{}] type 'a {}' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t type {} = tags {}'",
                variants, name, name, name
            )
        } else {
            let ty_name = rename.expect("bug in ocaml-gen: rename should be Some");
            env.add_alias(ty_id, ty_name);

            format!(
                "type 'a {}' = 'a {}' type {} = {}",
                ty_name, name, ty_name, name
            )
        }
    }
}

// Static assertions to verify that DynBox<T> is Sync and Send
assert_not_impl_all!(std::cell::RefCell<i32>: Sync); // RefCell<i32> is not Sync
assert_impl_all!(DynBox<std::cell::RefCell<i32>>: Sync, Send); // But DynBox allows RefCell<i32>
assert_impl_all!(DynBox<i32>: Sync, Send); // And DynBox allows Sync + Send obviously

/// A thin wrapper around a pointer to `DynArc`.
/// We "leak" `Arc` into a raw pointer to hold that raw pointer in the OCaml
/// heap, ensuring that moving of that value by the OCaml GC does not affect any
/// Rust invariants.
struct RustyObj(*const (dyn Any + Send + Sync));

/// Finalizer is registered with OCaml GC, and ensures that our "leaked" `Arc`
/// pointer is properly cleaned-up whenever OCaml drops corresponding object
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
        // orig_dynbox is owned by OCaml GC at this moment, so we can't drop it
        // from Rust
        let _ = std::mem::ManuallyDrop::new(orig_dynbox);
        // dynbox is owned by Rust as a valid Arc clone, so we should be good to
        // go to use it. Even if OCaml GC drops the original dynbox reference,
        // we will proceed with our own
        dynbox
    }
}

unsafe impl<T> ocaml::ToValue for DynBox<T>
where
    T: Send + ?Sized + 'static,
{
    fn to_value(&self, rt: &ocaml::Runtime) -> ocaml::Value {
        // Do a fresh clone of self and turn that into raw pointer
        let ptr = DynBox::into_raw(self.clone());
        // Convert to RustyObj to ensure that finalizer will be associated with
        // raw Arc pointer
        let rusty_obj = RustyObj(ptr);
        ocaml::Pointer::from(rusty_obj).to_value(rt)
    }
}

impl<T> From<T> for DynBox<T>
where
    T: Send + 'static,
{
    /// Default From implementation is just creating an exclusive DynBox, i.e.
    /// protected by a Mutex, be careful with deadlocks!
    fn from(value: T) -> Self {
        DynBox::new_exclusive(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as ocaml_rs_smartptr; // For proc macro use below to work
    use crate::{register_trait, register_type};
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

    #[test]
    #[serial(registry)]
    fn test_error_boxing() {
        register_trait!({
            ty: std::error::Error,
            marker_traits: [core::marker::Send],
        });
        let error = MyError {
            msg: String::from("bla-bla-bla"),
        };
        let orig_error_msg = error.to_string();
        let error: DynBox<dyn std::error::Error + Send> = error.into();
        // The following line mimics the dynbox being sent to OCaml and received
        // back as another type
        let error = DynBox::from_raw(DynBox::into_raw(error));
        let wrapped_error_msg = get_error_message(error);
        assert_eq!(wrapped_error_msg, orig_error_msg);
    }

    // Unfortunately supertrait support does not work yet with stable Rust :(
    // rust: cannot cast `dyn Error` to `dyn Display`, trait upcasting coercion is experimental
    // see issue #65991 <https://github.com/rust-lang/rust/issues/65991> for more information
    // required when coercing `&dyn Error` into `&dyn Display`
    // #[test]
    // #[serial(registry)]
    // fn test_dyn_bla() {
    //     register_trait!({
    //         ty: crate::ptr::tests::ErrorCombined,
    //         marker_traits: [core::marker::Sync, core::marker::Send],
    //         super_traits: [std::fmt::Display, core::fmt::Debug],
    //     });
    //     let error = MyError {
    //         msg: String::from("bla-bla-bla"),
    //     };
    //     let orig_error_msg = error.to_string();
    //     let error = DynBox::new_shared(error);
    //     // The following line mimics the dynbox being sent to OCaml and received
    //     // back as another type
    //     let error = DynBox::from_raw(DynBox::into_raw(error));
    //     let wrapped_error_msg = get_error_message(error);
    //     assert_eq!(wrapped_error_msg, orig_error_msg);
    // }
}
