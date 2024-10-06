use std::panic::UnwindSafe;
use std::sync::Arc;

use derive_more::derive::Display;

/// It's safe to send ocaml::root::Root across threads as long as access to
/// OCaml domain is properly synchronized. This wrapper type allows to send
/// ocaml::root::Root and provides a safe interface for doing so - value can be
/// recovered back only in context where OCaml runtime handle is available.
/// As cloning of `ocaml::root::Root` is not safe outside of OCaml Domain lock
/// (see comments in `as_value` method below), we wrap it with Arc to enable
/// safe cloning of MlBox from Rust
#[derive(Clone, Debug, Display)]
#[display("MlBox<{:?}>", inner)]
pub struct MlBox {
    inner: Arc<ocaml::root::Root>,
}

/* box root is just a pointer, wrapped by Arc, so MlBox is thus safe to send to
 * other threads */
unsafe impl Send for MlBox {}
/* all methods of MlBox require OCaml runtime, and thus can not be concurrently
 * run from different threads, making MlBox Sync, the only exception is .clone(),
 * but that is handled by Arc, so it's perfectly Sync too */
unsafe impl Sync for MlBox {}
impl UnwindSafe for MlBox {}

impl MlBox {
    /// Creates a new MlBox out of ocaml::Value, takes OCaml runtime handle to
    /// ensure this operation is called while OCaml domain lock is acquired
    pub fn new(_gc: &ocaml::Runtime, value: ocaml::Value) -> Self {
        match value {
            ocaml::Value::Raw(v) => {
                /* ocaml::Value was a raw one, need to create a new root for it
                 * to avoid it from being garbage collected by the OCaml GC */
                Self {
                    #[allow(clippy::arc_with_non_send_sync)]
                    inner: Arc::new(unsafe { ocaml::root::Root::new(v) }),
                }
            }
            ocaml::Value::Root(r) => {
                /* ocaml::Value was already rooted, so we can just take the root
                 * ouf of it and safely proceed with it further */
                Self {
                    #[allow(clippy::arc_with_non_send_sync)]
                    inner: Arc::new(r),
                }
            }
        }
    }

    /// Consumes this MlBox to recover original ocaml::Value (it will be a
    /// rooted one) if internal Arc was the only strong reference, otherwise
    /// returns None. Generally using `as_value` is more convenient. This method
    /// can be used when you're sure that you have only one reference to MlBox,
    /// in this case using this method can save on new boxroot allocation.
    pub fn into_value(self, _gc: &ocaml::Runtime) -> Option<ocaml::Value> {
        Arc::into_inner(self.inner).map(ocaml::Value::Root)
    }

    /// Creates a new rooted ocaml::Value, root is obtained by recovering value
    /// from current root and creating a new root for it
    pub fn as_value(&self, _gc: &ocaml::Runtime) -> ocaml::Value {
        // Caveat: we call .clone() on `ocaml::root::Root`, which will create a
        // new boxroot with value, obtained from current boxroot. According to
        // `boxroot.h`, both `boxroot_create` and `boxroot_get` require OCaml
        // domain lock to be held by current thread. I.e. calling .clone() on
        // `ocaml::root::Root` is generally not safe to do unless OCaml domain
        // lock is held - hence this function ensures that we have OCaml runtime
        // reference to maintain the safety guarantees
        let new_root = self.inner.as_ref().clone();
        ocaml::Value::Root(new_root)
    }
}
