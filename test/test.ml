(* Hand-written layer of bindings might look like this: *)

module Animal = struct
  include Stubs.Animal
end

module Sheep = struct
  include Animal
  include Stubs.Sheep
end

module Wolf = struct
  include Animal
  include Stubs.Wolf
end

module Test_callback = struct
  include Stubs.Test_callback
end

(* Now use hand-written bindings in actual code: *)

let sheep_test () =
  print_endline "\n*** Sheep test";
  let sheep = Sheep.create "dolly" in
  Animal.talk sheep;
  Sheep.sheer sheep;
  (* inclusion of Animal into Sheep allows to call Animal methods on Sheep right
  from Sheep module for convenience *)
  Sheep.talk sheep
;;

let wolf_test () =
  print_endline "\n*** Wolf test";
  let wolf = Wolf.create "big bad wolf" in
  Animal.talk wolf;
  let animal =
    Test_callback.call_cb wolf (fun wolf ->
      print_endline "(wolf gets modified inside a callback!)";
      Gc.full_major ();
      Wolf.set_hungry wolf true;
      Gc.full_major ();
      wolf)
  in
  Animal.talk animal
;;

let main () =
  sheep_test ();
  wolf_test ()
;;

let () = main ()
