//! This module provides a `Registry` for managing type coercions and type information
//! in a Rust-OCaml binding context. The `Registry` allows for the registration of
//! coercion functions that convert between different types, leveraging Rust's type
//! system and OCaml's garbage collector.
//!
//! The primary goal is to facilitate the interaction between Rust and OCaml by
//! providing a mechanism to register and retrieve type information and coercion
//! functions. This is particularly useful when dealing with dynamic types (`dyn Any`)
//! and ensuring that the OCaml garbage collector can properly manage Rust objects.
//!
//! The `Registry` uses `TypeId` to uniquely identify types and `DynArc` to store
//! dynamically typed values. Coercion functions are stored in a `HashMap` and can be
//! retrieved to convert between registered types.
//!
//! See relevant discussion: <https://users.rust-lang.org/t/rust-ocaml-bindings-and-traits/113263>
//! Special thanks to Kevin Reid (<https://users.rust-lang.org/u/kpreid>) for
//! providing the basis for building this module.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{
    Arc, Mutex, MutexGuard, Once, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use owning_ref::{ErasedBoxRef, ErasedBoxRefMut, OwningHandle, OwningRef, OwningRefMut};

/// An enum representing a read guard for either a `Mutex` or `RwLock`.
/// This allows for a unified interface for read access to the underlying data.
enum LockReadGuard<'a, T> {
    Mutex(MutexGuard<'a, T>),
    RwLockRead(RwLockReadGuard<'a, T>),
}

impl<T> Deref for LockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            LockReadGuard::Mutex(guard) => guard,
            LockReadGuard::RwLockRead(guard) => guard,
        }
    }
}

/// Both MutexGuard and RwLockReadGuard are StableDeref, so enum of those two is
/// also StableDeref
unsafe impl<T> stable_deref_trait::StableDeref for LockReadGuard<'_, T> {}

/// An enum representing a write guard for either a `Mutex` or `RwLock`.
/// This allows for a unified interface for write access to the underlying data.
enum LockWriteGuard<'a, T> {
    Mutex(MutexGuard<'a, T>),
    RwLockWrite(RwLockWriteGuard<'a, T>),
}

impl<T> Deref for LockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            LockWriteGuard::Mutex(guard) => guard,
            LockWriteGuard::RwLockWrite(guard) => guard,
        }
    }
}

impl<T> DerefMut for LockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            LockWriteGuard::Mutex(guard) => &mut *guard,
            LockWriteGuard::RwLockWrite(guard) => &mut *guard,
        }
    }
}

/// Both MutexGuard and RwLockWriteGuard are StableDeref, so enum of those two
/// is also StableDeref
unsafe impl<T> stable_deref_trait::StableDeref for LockWriteGuard<'_, T> {}

/// A type alias for an `Arc` containing a dynamically typed value that is both
/// `Sync` and `Send`. This is used to store values in the registry.
type DynArc = Arc<dyn Any + Sync + Send>;

/// Type alias for a function that takes a `DynArc` and returns a boxed `dyn Any`.
/// This is used for type coercion in the registry.
type CoercionInAny = Arc<dyn Fn(DynArc) -> Box<dyn Any> + Sync + Send>;

/// A type alias for a handle to a read-only reference of type `Out`.
/// This is used to represent coerced values in the registry.
pub type Handle<Out> = ErasedBoxRef<Out>; // Holds a lock on DynArc

/// A type alias for a handle to a mutable reference of type `Out`.
/// This is used to represent coerced mutable values in the registry.
pub type HandleMut<Out> = ErasedBoxRefMut<Out>; // Holds a lock on DynArc

/// A struct representing type information, including the fully qualified name
/// and a list of implementations.
#[derive(Clone)]
pub struct TypeInfo {
    pub fq_name: &'static str,
    pub implementations: Vec<&'static str>,
}

/// The `Registry` struct holds mappings for type coercions and type information.
/// It allows registering coercion functions for converting between types and
/// retrieving type information.
#[derive(Default)]
struct Registry {
    traits: HashMap<(TypeId, TypeId), (CoercionInAny, CoercionInAny)>,
    types: HashMap<TypeId, String>,
    type_info_map: HashMap<TypeId, TypeInfo>,
}

impl Registry {
    /// Creates a new `Registry` instance.
    ///
    /// # Returns
    ///
    /// A new `Registry` instance.
    fn new() -> Self {
        Self::default()
    }

    /// Registers coercion functions for converting between types `In` and `Out`.
    ///
    /// # Parameters
    ///
    /// - `fs`: A tuple containing two `CoercionInAny` functions for read and write coercions.
    fn register_coercion_fns<In: Sized + 'static, Out: ?Sized + 'static>(
        &mut self,
        fs: (CoercionInAny, CoercionInAny),
    ) {
        self.traits
            .insert((TypeId::of::<In>(), TypeId::of::<Out>()), fs);
    }

    /// Registers a type in the registry.
    ///
    /// # Parameters
    ///
    /// - `In`: The trait object type to register.
    /// - `InReal`: The real type that implements the trait.
    fn register_type<In: ?Sized + 'static, InReal: ?Sized>(&mut self) {
        self.types
            .insert(TypeId::of::<In>(), std::any::type_name::<InReal>().into());
    }

    /// Registers type information in the registry.
    ///
    /// # Parameters
    ///
    /// - `In`: The trait object type to register.
    /// - `fq_name`: The fully qualified name of the type.
    /// - `impls`: A vector of strings representing the implementations of the type.
    fn register_type_info<In: ?Sized + 'static>(
        &mut self,
        fq_name: &'static str,
        impls: Vec<&'static str>,
    ) {
        self.type_info_map.insert(
            TypeId::of::<In>(),
            TypeInfo {
                fq_name,
                implementations: impls,
            },
        );
    }

    /// Registers coercion functions for converting between types `In` and `Out`.
    ///
    /// # Parameters
    ///
    /// - `conv`: A function pointer for read coercion.
    /// - `conv_mut`: A function pointer for write coercion.
    fn register<In: Sized + 'static, Out: ?Sized + 'static>(
        &mut self,
        conv: fn(&In) -> &Out,
        conv_mut: fn(&mut In) -> &mut Out,
    ) {
        // Retrieve the type name for the input type.
        let type_in_name = String::from(self.type_name(&TypeId::of::<In>()));
        // Clone the type name for use in the mutable coercion function.
        let type_in_name_mut = type_in_name.clone();
        // Create the read coercion function.
        let f: CoercionInAny = Arc::new(move |boxed_t: DynArc| {
            let ohandle = OwningHandle::new_with_fn(boxed_t, |bt| {
                let any = unsafe { bt.as_ref() }.unwrap();
                let guard = if let Some(mutex) = any.downcast_ref::<Mutex<In>>() {
                    LockReadGuard::Mutex(mutex.lock().unwrap())
                } else if let Some(rwlock) = any.downcast_ref::<RwLock<In>>() {
                    LockReadGuard::RwLockRead(rwlock.read().unwrap())
                } else {
                    panic!(
                        "unsupported container provided for coersion (type: {:?})",
                        type_in_name
                    );
                };
                OwningRef::new(guard).map(conv)
            });
            Box::new(OwningRef::new(ohandle).map_owner_box().erase_owner())
        });
        // Create the write coercion function.
        let f_mut: CoercionInAny = Arc::new(move |boxed_t: DynArc| {
            let ohandle = OwningHandle::new_with_fn(boxed_t, |bt| {
                let any = unsafe { bt.as_ref() }.unwrap();
                let guard = if let Some(mutex) = any.downcast_ref::<Mutex<In>>() {
                    LockWriteGuard::Mutex(mutex.lock().unwrap())
                } else if let Some(rwlock) = any.downcast_ref::<RwLock<In>>() {
                    LockWriteGuard::RwLockWrite(rwlock.write().unwrap())
                } else {
                    panic!(
                        "unsupported container provided for mut coersion (type: {:?})",
                        type_in_name_mut
                    );
                };
                OwningRefMut::new(guard).map_mut(conv_mut)
            });
            Box::new(OwningRefMut::new(ohandle).map_owner_box().erase_owner())
        });
        // Clone the coercion functions for registration.
        let clone = || (f.clone(), f_mut.clone());
        // Register the coercion functions for `Mutex<In>` to `Out`.
        self.register_coercion_fns::<Mutex<In>, Out>(clone());
        // Register the coercion functions for `RwLock<In>` to `Out`.
        self.register_coercion_fns::<RwLock<In>, Out>(clone());
    }

    /// Retrieves the coercion functions for a given output type.
    ///
    /// # Parameters
    ///
    /// - `input`: A reference to a `DynArc` input.
    ///
    /// # Returns
    ///
    /// A tuple containing two `CoercionInAny` functions for read and write coercions.
    fn get_coerce_fns<Out: ?Sized + 'static>(
        &self,
        input: &DynArc,
    ) -> &(CoercionInAny, CoercionInAny) {
        // Retrieve the `TypeId` of the input type.
        // `**` is for: &Arc<dyn Any> -> Arc<dyn Any> -> dyn Any
        let type_in = (**input).type_id();
        // Retrieve the `TypeId` of the output type.
        let type_out = TypeId::of::<Out>();
        // Retrieve the type name for the input type.
        let type_in_name = self.type_name(&type_in);
        // Retrieve the coercion functions from the registry.
        self.traits.get(&(type_in, type_out)).unwrap_or_else(|| {
            panic!(
                "there is no registered coercion for {:?} => {:?}",
                type_in_name,
                std::any::type_name::<Out>()
            )
        })
    }

    /// Retrieves the type name for a given `TypeId`.
    ///
    /// # Parameters
    ///
    /// - `type_in`: A reference to a `TypeId`.
    ///
    /// # Returns
    ///
    /// A string slice representing the type name.
    fn type_name(&self, type_in: &TypeId) -> &str {
        // Retrieve the type name from the registry.
        match self.types.get(type_in) {
            Some(name) => name.as_str(),
            None => "<unregistered type>",
        }
    }

    /// Coerces a `DynArc` input to a handle of the specified output type.
    ///
    /// # Parameters
    ///
    /// - `input`: A `DynArc` input.
    ///
    /// # Returns
    ///
    /// A handle to the coerced output type.
    fn coerce<Out: ?Sized + 'static>(&self, input: DynArc) -> Handle<Out> {
        // Retrieve the read coercion function.
        let (f, _) = self.get_coerce_fns::<Out>(&input);
        // Coerce the input to the output type.
        // Coerce the input to the mutable output type.
        *f(input.clone())
            .downcast()
            .expect("coercion fn returned wrong type")
    }

    /// Coerces a `DynArc` input to a mutable handle of the specified output type.
    ///
    /// # Parameters
    ///
    /// - `input`: A `DynArc` input.
    ///
    /// # Returns
    ///
    /// A mutable handle to the coerced output type.
    fn coerce_mut<Out: ?Sized + 'static>(&self, input: DynArc) -> HandleMut<Out> {
        // Retrieve the write coercion function.
        let (_, f) = self.get_coerce_fns::<Out>(&input);
        *f(input.clone())
            .downcast()
            .expect("coercion fn returned wrong type")
    }

    /// Retrieves the type information for a given input type.
    ///
    /// # Parameters
    ///
    /// - `In`: The trait object type to retrieve information for.
    ///
    /// # Returns
    ///
    /// A `TypeInfo` struct containing the type information.
    fn get_type_info<In: ?Sized + 'static>(&self) -> TypeInfo {
        // Retrieve the `TypeId` of the input type.
        let type_id = TypeId::of::<In>();
        // Retrieve the type information from the registry.
        let type_info = self.type_info_map.get(&type_id).unwrap_or_else(|| {
            panic!(
                "registry does not contain a registered type info for {}",
                std::any::type_name::<In>()
            )
        });
        type_info.clone()
    }
}

/// Returns a reference to the global registry.
///
/// # Returns
///
/// A reference to the global `RwLock<Registry>`.
fn global_registry() -> &'static RwLock<Registry> {
    // Initialize the global registry.
    static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Registry::new()))
}

/// Registers coercion functions for converting between types `In` and `Out` in the global registry.
///
/// # Parameters
///
/// - `conv`: A function pointer for read coercion.
/// - `conv_mut`: A function pointer for write coercion.
pub fn register<In: Sized + 'static, Out: ?Sized + 'static>(
    conv: fn(&In) -> &Out,
    conv_mut: fn(&mut In) -> &mut Out,
) {
    // Obtain a write lock on the global registry.
    let mut registry = global_registry()
        .write()
        .expect("unable to obtain write lock on global registry");
    registry.register::<In, Out>(conv, conv_mut)
}

/// Registers a type in the global registry.
///
/// # Parameters
///
/// - `In`: The trait object type to register.
pub fn register_type<In: ?Sized + 'static>() {
    let mut registry = global_registry()
        .write()
        .expect("unable to obtain write lock on global registry");
    registry.register_type::<In, In>();
    registry.register_type::<Mutex<In>, In>();
    registry.register_type::<RwLock<In>, In>();
}

/// Registers type information in the global registry.
///
/// # Parameters
///
/// - `In`: The trait object type to register.
/// - `fq_name`: The fully qualified name of the type.
/// - `impls`: A vector of strings representing the implementations of the type.
pub fn register_type_info<In: ?Sized + 'static>(
    fq_name: &'static str,
    impls: Vec<&'static str>,
) {
    let mut registry = global_registry()
        .write()
        .expect("unable to obtain write lock on global registry");
    registry.register_type_info::<In>(fq_name, impls);
}

/// Coerces a `DynArc` input to a handle of the specified output type using the global registry.
///
/// # Parameters
///
/// - `input`: A `DynArc` input.
///
/// # Returns
///
/// A handle to the coerced output type.
pub fn coerce<Out: ?Sized + 'static>(input: DynArc) -> Handle<Out> {
    // Note: This function holds a lock on DynArc. Use with care to avoid deadlocks.
    // Obtain a read lock on the global registry.
    let registry = global_registry()
        .read()
        .expect("unable to obtain read lock on global registry");
    registry.coerce::<Out>(input)
}

/// Coerces a `DynArc` input to a mutable handle of the specified output type using the global registry.
///
/// # Parameters
///
/// - `input`: A `DynArc` input.
///
/// # Returns
///
/// A mutable handle to the coerced output type.
pub fn coerce_mut<Out: ?Sized + 'static>(input: DynArc) -> HandleMut<Out> {
    // Note: This function holds a lock on DynArc. Use with care to avoid deadlocks.
    let registry = global_registry()
        .read()
        .expect("unable to obtain read lock on global registry");
    registry.coerce_mut::<Out>(input)
}

/// Retrieves the type information for a given input type from the global registry.
///
/// # Parameters
///
/// - `In`: The trait object type to retrieve information for.
///
/// # Returns
///
/// A `TypeInfo` struct containing the type information.
pub fn get_type_info<In: ?Sized + 'static>() -> TypeInfo {
    let registry = global_registry()
        .read()
        .expect("unable to obtain read lock on global registry");
    registry.get_type_info::<In>()
}

/// The `Plugin` struct represents a plugin with an initializer function.
pub struct Plugin {
    /// A function pointer to the initializer function.
    initializer: fn(),
}

impl Plugin {
    /// Creates a new `Plugin` with the given initializer function.
    ///
    /// # Parameters
    ///
    /// - `initializer`: A function pointer to the initializer function.
    ///
    /// # Returns
    ///
    /// A new `Plugin` instance.
    pub const fn new(initializer: fn()) -> Self {
        // Create a new `Plugin` instance with the given initializer function.
        Plugin { initializer }
    }

    /// Initializes the plugin by calling its initializer function.
    fn initialize(&self) {
        // Call the initializer function.
        (self.initializer)();
    }
}

inventory::collect!(Plugin);

static INIT: Once = Once::new();

/// Initializes all registered plugins. This function is called once.
pub fn initialize_plugins() {
    // Initialize all registered plugins.
    INIT.call_once(|| {
        for plugin in inventory::iter::<Plugin> {
            plugin.initialize();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    macro_rules! register_trait {
        ($type:ty, $($trait:tt)+) => {
            $crate::registry::register_type::<$type>();
            $crate::registry::register_type::<$($trait)+>();
            $crate::registry::register::<$type, $($trait)+>(
                |x: &$type| x as &($($trait)+),
                |x: &mut $type| x as &mut ($($trait)+)
            );
        };
    }

    fn reinit_global_registry() {
        let mut registry = global_registry().write().unwrap();
        *registry = Registry::new()
    }

    trait Foo {
        fn bar(&self) -> String;
    }

    impl Foo for i32 {
        fn bar(&self) -> String {
            format!("Foo for i32 ({:?})", self)
        }
    }

    impl Foo for String {
        fn bar(&self) -> String {
            format!("Foo for String ({:?})", self)
        }
    }

    trait FooMut {
        fn bar_mut(&mut self) -> String;
    }

    impl FooMut for i32 {
        fn bar_mut(&mut self) -> String {
            *self += 1;
            format!("FooMut for i32 ({:?})", self)
        }
    }

    impl FooMut for String {
        fn bar_mut(&mut self) -> String {
            self.push('!');
            format!("FooMut for String ({:?})", self)
        }
    }

    fn test_display(values: Vec<DynArc>) -> Vec<String> {
        let mut results = Vec::new();
        for value in values {
            let coerced = coerce::<dyn std::fmt::Display>(value);
            let coerced = coerced.deref();
            results.push(format!("{coerced}"));
        }
        results
    }

    fn test_foo(values: Vec<DynArc>) -> Vec<String> {
        let mut results = Vec::new();
        for value in values {
            let coerced = coerce::<dyn Foo>(value);
            let coerced = coerced.deref();
            results.push(coerced.bar());
        }
        results
    }

    fn test_foo_send(values: Vec<DynArc>) -> Vec<String> {
        let mut results = Vec::new();
        for value in values {
            let coerced = coerce::<dyn Foo + Send>(value);
            let coerced = coerced.deref();
            results.push(coerced.bar());
        }
        results
    }

    fn test_foo_mut(values: Vec<DynArc>) -> Vec<String> {
        let mut results = Vec::new();
        for value in values {
            let mut coerced = coerce_mut::<dyn FooMut>(value);
            results.push(coerced.bar_mut());
        }
        results
    }

    #[test]
    #[serial(registry)]
    fn test_registry_display() {
        reinit_global_registry();
        register_trait!(i32, dyn std::fmt::Display);
        register_trait!(i32, dyn core::fmt::Debug);
        register_trait!(i32, dyn Foo);
        register_trait!(String, dyn std::fmt::Display);
        register_trait!(String, dyn core::fmt::Debug);
        register_trait!(String, dyn Foo);

        let values: Vec<DynArc> = vec![
            Arc::new(Mutex::new(1)),
            Arc::new(RwLock::new(String::from("two"))),
        ];
        let results = test_display(values);

        assert_eq!(results, vec!["1", "two"]);
    }

    #[test]
    #[serial(registry)]
    fn test_registry_foo() {
        reinit_global_registry();
        register_trait!(i32, dyn std::fmt::Display);
        register_trait!(i32, dyn core::fmt::Debug);
        register_trait!(i32, dyn Foo);
        register_trait!(String, dyn std::fmt::Display);
        register_trait!(String, dyn core::fmt::Debug);
        register_trait!(String, dyn Foo);

        let values: Vec<DynArc> = vec![
            Arc::new(Mutex::new(3)),
            Arc::new(RwLock::new(String::from("four"))),
        ];
        let results = test_foo(values);

        assert_eq!(
            results,
            vec!["Foo for i32 (3)", "Foo for String (\"four\")"]
        );
    }

    #[test]
    #[serial(registry)]
    fn test_registry_compound_trait() {
        reinit_global_registry();
        register_trait!(i32, dyn std::fmt::Display);
        register_trait!(i32, dyn core::fmt::Debug);
        register_trait!(i32, dyn Foo);
        register_trait!(i32, dyn Foo + Send);
        register_trait!(String, dyn std::fmt::Display);
        register_trait!(String, dyn core::fmt::Debug);
        register_trait!(String, dyn Foo);
        register_trait!(String, dyn Foo + Send);

        let values: Vec<DynArc> = vec![
            Arc::new(Mutex::new(3)),
            Arc::new(RwLock::new(String::from("four"))),
        ];
        let results = test_foo_send(values);

        assert_eq!(
            results,
            vec!["Foo for i32 (3)", "Foo for String (\"four\")"]
        );
    }

    #[test]
    #[serial(registry)]
    fn test_registry_foo_mut() {
        reinit_global_registry();
        register_trait!(i32, dyn FooMut);
        register_trait!(String, dyn FooMut);

        let values: Vec<DynArc> = vec![
            Arc::new(Mutex::new(3)),
            Arc::new(RwLock::new(String::from("four"))),
        ];
        let results = test_foo_mut(values);

        assert_eq!(
            results,
            vec!["FooMut for i32 (4)", "FooMut for String (\"four!\")"]
        );
    }
}
