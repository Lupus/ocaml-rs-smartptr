open Test_lib.Bindings

let main () =
  let sheep = Sheep.create "dolly" in
  Animal.talk (sheep :> Animal.t);
  Sheep.sheer sheep;
  Animal.talk (sheep :> Animal.t)
;;

let () = main ()
