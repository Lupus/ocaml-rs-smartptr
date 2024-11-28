(lang dune 3.7)

(name ocaml-rs-smartptr)

(generate_opam_files true)

(source
 (github Lupus/ocaml-rs-smartptr))

(license Apache-2.0)

(authors "Konstantin Olkhovskiy <lupus@oxnull.net>")

(maintainers "Konstantin Olkhovskiy <lupus@oxnull.net>")

(package
 (name ocaml-rs-smartptr)
 (synopsis "Smart pointer for ocaml-rs")
 (depends
  (rust-staticlib-gen
   (and
    (>= 0.2.2)
    :with-test))
  (rust-staticlib-virtual
   (>= 0.2.2))
  (dune-cargo-build
   (and
    (>= 0.2.2)
    :build))
  (ocamlformat
   (and
    :with-test
    (>= 0.26.2)
    (< 0.27.0)))
  (odoc :with-doc)))
