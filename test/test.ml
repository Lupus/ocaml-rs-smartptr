open Stubs

let sheep_test () =
  print_endline "\n*** Sheep test";
  let sheep = Sheep.create "dolly" in
  Animal.talk (sheep :> Animal.t);
  Sheep.sheer sheep;
  Animal.talk (sheep :> Animal.t)
;;

let wolf_test () =
  print_endline "\n*** Wolf test";
  let wolf = Wolf.create "big bad wolf" in
  Animal.talk (wolf :> Animal.t);
  let animal =
    Test_callback.call_cb wolf (fun wolf ->
      print_endline "(wolf gets modified inside a callback!)";
      Gc.full_major ();
      Wolf.set_hungry wolf true;
      Gc.full_major ();
      (wolf :> Animal.t))
  in
  Animal.talk animal
;;

let main () =
  sheep_test ();
  wolf_test ()
;;

let () = main ()
