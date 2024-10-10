
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


module Test_callback = struct 
  external call_cb : Wolf.t -> ((Wolf.t) -> (Animal.t)) -> Animal.t = "call_cb"
end

