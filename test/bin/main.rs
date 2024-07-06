use ocaml_gen::prelude::*;
use ocaml_rs_smartptr::ptr::DynBox;
use ocaml_rs_smartptr_test::stubs::*;

use std::fmt::Write as _;
use std::io;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut w = String::new();
    let env = &mut Env::new();

    ocaml_gen::decl_fake_generic!(T1, 0);

    ocaml_gen::decl_module!(w, env, "Animal", {
        ocaml_gen::decl_type!(w, env, DynBox<Animal> => "t");
        ocaml_gen::decl_func!(w, env, animal_name => "name");
        ocaml_gen::decl_func!(w, env, animal_noise => "noise");
        ocaml_gen::decl_func!(w, env, animal_talk => "talk");
    });

    ocaml_gen::decl_module!(w, env, "Sheep", {
        ocaml_gen::decl_type!(w, env, DynBox<Sheep> => "t");
        ocaml_gen::decl_func!(w, env, sheep_create => "create");
        ocaml_gen::decl_func!(w, env, sheep_is_naked => "is_naked");
        ocaml_gen::decl_func!(w, env, sheep_sheer => "sheer");
    });

    ocaml_gen::decl_module!(w, env, "Wolf", {
        ocaml_gen::decl_type!(w, env, DynBox<Wolf> => "t");
        ocaml_gen::decl_func!(w, env, wolf_create => "create");
    });

    io::stdout().write_all(w.as_bytes())?;
    Ok(())
}
