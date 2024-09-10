use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::{
    Arc, Mutex, MutexGuard, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use owning_ref::{ErasedBoxRef, ErasedBoxRefMut, OwningHandle, OwningRef, OwningRefMut};

enum LockReadGuard<'a, T> {
    Mutex(MutexGuard<'a, T>),
    RwLockRead(RwLockReadGuard<'a, T>),
}

impl<'a, T> Deref for LockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            LockReadGuard::Mutex(guard) => guard,
            LockReadGuard::RwLockRead(guard) => guard,
        }
    }
}

/* Both MutexGuard and RwLockReadGuard are StableDeref, so enum of those two is
 * also StableDeref */
unsafe impl<'a, T> stable_deref_trait::StableDeref for LockReadGuard<'a, T> {}

enum LockWriteGuard<'a, T> {
    Mutex(MutexGuard<'a, T>),
    RwLockWrite(RwLockWriteGuard<'a, T>),
}

impl<'a, T> Deref for LockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            LockWriteGuard::Mutex(guard) => guard,
            LockWriteGuard::RwLockWrite(guard) => guard,
        }
    }
}

impl<'a, T> DerefMut for LockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            LockWriteGuard::Mutex(guard) => &mut *guard,
            LockWriteGuard::RwLockWrite(guard) => &mut *guard,
        }
    }
}

/* Both MutexGuard and RwLockWriteGuard are StableDeref, so enum of those two is
 * also StableDeref */
unsafe impl<'a, T> stable_deref_trait::StableDeref for LockWriteGuard<'a, T> {}

type DynArc = Arc<dyn Any + Sync + Send>;

type CoercionInAny = Arc<dyn Fn(DynArc) -> Box<dyn Any> + Sync + Send>;

pub type Handle<Out> = ErasedBoxRef<Out>;
pub type HandleMut<Out> = ErasedBoxRefMut<Out>;

#[derive(Clone)]
pub struct TypeInfo {
    pub fq_name: &'static str,
    pub implementations: Vec<&'static str>,
}

#[derive(Default)]
struct Registry {
    traits: HashMap<(TypeId, TypeId), (CoercionInAny, CoercionInAny)>,
    types: HashMap<TypeId, String>,
    type_info_map: HashMap<TypeId, TypeInfo>,
}

impl Registry {
    fn new() -> Self {
        Self::default()
    }

    fn register_coercion_fns<In: Sized + 'static, Out: ?Sized + 'static>(
        &mut self,
        fs: (CoercionInAny, CoercionInAny),
    ) {
        self.traits
            .insert((TypeId::of::<In>(), TypeId::of::<Out>()), fs);
    }

    fn register_type<In: ?Sized + 'static, InReal: ?Sized>(&mut self) {
        self.types
            .insert(TypeId::of::<In>(), std::any::type_name::<InReal>().into());
    }

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

    fn register<In: Sized + 'static, Out: ?Sized + 'static>(
        &mut self,
        conv: fn(&In) -> &Out,
        conv_mut: fn(&mut In) -> &mut Out,
    ) {
        let type_in_name = String::from(self.type_name(&TypeId::of::<In>()));
        let type_in_name_mut = type_in_name.clone();
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
        let clone = || (f.clone(), f_mut.clone());
        self.register_coercion_fns::<Mutex<In>, Out>(clone());
        self.register_coercion_fns::<RwLock<In>, Out>(clone());
    }

    fn get_coerce_fns<Out: ?Sized + 'static>(
        &self,
        input: &DynArc,
    ) -> &(CoercionInAny, CoercionInAny) {
        let type_in = (**input).type_id(); // &Arc<dyn Any> -> Arc<dyn Any> -> dyn Any
        let type_out = TypeId::of::<Out>();
        let type_in_name = self.type_name(&type_in);
        self.traits.get(&(type_in, type_out)).unwrap_or_else(|| {
            panic!(
                "there is no registered coercion for {:?} => {:?}",
                type_in_name,
                std::any::type_name::<Out>()
            )
        })
    }

    fn type_name(&self, type_in: &TypeId) -> &str {
        match self.types.get(type_in) {
            Some(name) => name.as_str(),
            None => "<unregistered type>",
        }
    }

    fn coerce<Out: ?Sized + 'static>(&self, input: DynArc) -> Handle<Out> {
        let (f, _) = self.get_coerce_fns::<Out>(&input);
        *f(input.clone())
            .downcast()
            .expect("coercion fn returned wrong type")
    }

    fn coerce_mut<Out: ?Sized + 'static>(&self, input: DynArc) -> HandleMut<Out> {
        let (_, f) = self.get_coerce_fns::<Out>(&input);
        *f(input.clone())
            .downcast()
            .expect("coercion fn returned wrong type")
    }

    fn get_type_info<In: ?Sized + 'static>(&self) -> TypeInfo {
        let type_id = TypeId::of::<In>();
        let type_info = self.type_info_map.get(&type_id).unwrap_or_else(|| {
            panic!(
                "registry does not contain a registered type info for {}",
                std::any::type_name::<In>()
            )
        });
        type_info.clone()
    }
}

fn global_registry() -> &'static RwLock<Registry> {
    static REGISTRY: OnceLock<RwLock<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| RwLock::new(Registry::new()))
}

pub fn register<In: Sized + 'static, Out: ?Sized + 'static>(
    conv: fn(&In) -> &Out,
    conv_mut: fn(&mut In) -> &mut Out,
) {
    let mut registry = global_registry()
        .write()
        .expect("unable to obtain write lock on global registry");
    registry.register::<In, Out>(conv, conv_mut)
}

pub fn register_type<In: ?Sized + 'static>() {
    let mut registry = global_registry()
        .write()
        .expect("unable to obtain write lock on global registry");
    registry.register_type::<In, In>();
    registry.register_type::<Mutex<In>, In>();
    registry.register_type::<RwLock<In>, In>();
}

pub fn register_type_info<In: ?Sized + 'static>(
    fq_name: &'static str,
    impls: Vec<&'static str>,
) {
    let mut registry = global_registry()
        .write()
        .expect("unable to obtain write lock on global registry");
    registry.register_type_info::<In>(fq_name, impls);
}

pub fn coerce<Out: ?Sized + 'static>(input: DynArc) -> Handle<Out> {
    let registry = global_registry()
        .read()
        .expect("unable to obtain read lock on global registry");
    registry.coerce::<Out>(input)
}

pub fn coerce_mut<Out: ?Sized + 'static>(input: DynArc) -> HandleMut<Out> {
    let registry = global_registry()
        .read()
        .expect("unable to obtain read lock on global registry");
    registry.coerce_mut::<Out>(input)
}

pub fn get_type_info<In: ?Sized + 'static>() -> TypeInfo {
    let registry = global_registry()
        .read()
        .expect("unable to obtain read lock on global registry");
    registry.get_type_info::<In>()
}

#[macro_export]
macro_rules! register_type {
    ($type:ty) => {
        $crate::registry::register_type::<$type>();
        $crate::registry::register::<$type, $type>(|x: &$type| x, |x: &mut $type| x);
    };
}

#[macro_export]
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

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
