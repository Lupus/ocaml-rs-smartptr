//! This module provides a safe wrapper around `ocaml::Value` to allow
//! sending OCaml values between threads in Rust. The `MlBox` struct ensures
//! that the OCaml runtime handle is available when recovering the value,
//! maintaining safety.

use std::panic::{AssertUnwindSafe, RefUnwindSafe, UnwindSafe};
use std::sync::Arc;

use derive_more::derive::Display;

/// This wrapper type around `ocaml::root::Root` allows sending
/// `ocaml::root::Root` to other threads and provides a safe interface for doing
/// so - the value can be recovered back only in a context where an OCaml
/// runtime handle is available.  As cloning of `ocaml::root::Root` is not safe
/// outside of the OCaml Domain lock (see comments in the `as_value` method
/// below), we wrap it with `Arc` to enable safe cloning of `MlBox` from Rust.
///
/// `ocaml::root::Root` is a wrapper type around what's returned by the `boxroot_create()`
/// C function. For more details, refer to the [boxroot documentation](https://gitlab.com/ocaml-rust/ocaml-boxroot/-/blob/main/boxroot/boxroot.h).
#[derive(Clone, Debug, Display)]
#[display("MlBox<{:?}>", inner)]
pub struct MlBox {
    inner: Arc<AssertUnwindSafe<ocaml::root::Root>>,
}

/// The box root is just a pointer, wrapped by `Arc`, so `MlBox` is thus safe to send to
/// other threads.
unsafe impl Send for MlBox {}
/// All methods of `MlBox` require the OCaml runtime, and thus cannot be concurrently
/// run from different threads, making `MlBox` Sync. The only exception is `.clone()`,
/// but that is handled by `Arc`, so it's perfectly Sync too.
unsafe impl Sync for MlBox {}

assert_impl_all!(MlBox: Send, Sync, UnwindSafe, RefUnwindSafe);

impl MlBox {
    /// Creates a new `MlBox` out of `ocaml::Value`, taking an OCaml runtime handle to
    /// ensure this operation is called while the OCaml domain lock is acquired.
    pub fn new(_gc: &ocaml::Runtime, value: ocaml::Value) -> Self {
        match value {
            ocaml::Value::Raw(v) => {
                // `ocaml::Value` was a raw one, need to create a new root for it
                // to avoid it from being garbage collected by the OCaml GC.
                Self {
                    #[allow(clippy::arc_with_non_send_sync)]
                    inner: Arc::new(AssertUnwindSafe(unsafe {
                        ocaml::root::Root::new(v)
                    })),
                }
            }
            ocaml::Value::Root(r) => {
                // `ocaml::Value` was already rooted, so we can just take the root
                // out of it and safely proceed with it further.
                Self {
                    #[allow(clippy::arc_with_non_send_sync)]
                    inner: Arc::new(AssertUnwindSafe(r)),
                }
            }
        }
    }

    /// Consumes this `MlBox` to recover the original `ocaml::Value` (it will be a
    /// rooted one) if the internal `Arc` was the only strong reference, otherwise
    /// returns `None`. Generally, using `as_value` is more convenient. This method
    /// can be used when you're sure that you have only one reference to `MlBox`,
    /// in this case using this method can save on new boxroot allocation.
    pub fn into_value(self, _gc: &ocaml::Runtime) -> Option<ocaml::Value> {
        Arc::into_inner(self.inner)
            .map(|x| x.0)
            .map(ocaml::Value::Root)
    }

    /// Creates a new rooted `ocaml::Value`, the root is obtained by recovering the value
    /// from the current root and creating a new root for it.
    pub fn as_value(&self, _gc: &ocaml::Runtime) -> ocaml::Value {
        // Caveat: we call `.clone()` on `ocaml::root::Root`, which will create a
        // new boxroot with the value obtained from the current boxroot. According to
        // `boxroot.h`, both `boxroot_create` and `boxroot_get` require the OCaml
        // domain lock to be held by the current thread. I.e. calling `.clone()` on
        // `ocaml::root::Root` is generally not safe to do unless the OCaml domain
        // lock is held - hence this function ensures that we have an OCaml runtime
        // reference to maintain the safety guarantees.
        let AssertUnwindSafe(new_root): &AssertUnwindSafe<ocaml::root::Root> =
            self.inner.as_ref();
        ocaml::Value::Root(new_root.clone())
    }
}

unsafe impl ocaml::ToValue for MlBox {
    fn to_value(&self, gc: &ocaml::Runtime) -> ocaml::Value {
        self.as_value(gc)
    }
}
