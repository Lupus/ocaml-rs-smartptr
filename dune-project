(lang dune 3.7)

(name ocaml-rs-smartptr)

(generate_opam_files true)

(package
 (name ocaml-rs-smartptr)
 (maintainers "ocaml-rs-smartptr developers")
 (synopsis "Smart pointer for ocaml-rs")
 (depends
  (ocamlformat
   (and
    :with-test
    (>= 0.26.2)
    (< 0.27.0)))
  (odoc :with-doc)))
