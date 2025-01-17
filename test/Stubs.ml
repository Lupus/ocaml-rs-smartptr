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
  external create_random : string -> _ t' = "animal_create_random"
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

module Animal_alias = struct
  type 'a animal' = 'a Animal.t'
  type animal = Animal.t

  external create_random_animal : string -> _ animal' = "animal_create_random"
end

module Export_import = struct
  external barn_create : int32 -> Some_other_lib.Barn.t = "barn_create"

  type nonrec barn = Some_other_lib.Barn.t

  external barn_create_with_alias : int32 -> barn = "barn_create"

  external dynbox_with_animal_create
    :  string
    -> _ Some_other_lib.Animal.t'
    = "dynbox_with_animal_create"
end
