module Barn = struct
  type t
end

module Animal = struct
  type tags =
    [ `Ocaml_rs_smartptr_test_stubs_animal_proxy
    | `Core_marker_send
    ]

  type 'a t' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t
  type t = tags t'
end
