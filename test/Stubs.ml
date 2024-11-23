module Animal = struct
  type tags =
    [ `Ocaml_rs_smartptr_test_stubs_animal_proxy
    | `Core_marker_send
    ]

  type 'a t' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t
  type t = tags t'

  external name : _ t' -> string = "animal_name"
  external noise : _ t' -> string = "animal_noise"
  external talk : _ t' -> unit = "animal_talk"
end

module Sheep = struct
  type tags =
    [ `Ocaml_rs_smartptr_test_stubs_sheep
    | `Core_marker_sync
    | `Core_marker_send
    | `Ocaml_rs_smartptr_test_stubs_animal_proxy
    ]

  type 'a t' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t
  type t = tags t'

  external create : string -> _ t' = "sheep_create"
  external is_naked : _ t' -> bool = "sheep_is_naked"
  external sheer : _ t' -> unit = "sheep_sheer"
end

module Wolf = struct
  type tags =
    [ `Ocaml_rs_smartptr_test_stubs_wolf
    | `Core_marker_sync
    | `Core_marker_send
    | `Ocaml_rs_smartptr_test_stubs_animal_proxy
    ]

  type 'a t' = ([> tags ] as 'a) Ocaml_rs_smartptr.Rusty_obj.t
  type t = tags t'

  external create : string -> _ t' = "wolf_create"
  external set_hungry : _ t' -> bool -> unit = "wolf_set_hungry"
end

module Test_callback = struct
  external call_cb : _ Wolf.t' -> (_ Wolf.t' -> _ Animal.t') -> _ Animal.t' = "call_cb"
end
