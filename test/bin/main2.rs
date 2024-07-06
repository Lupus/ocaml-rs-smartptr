//use rustdoc_json::Crate;
use codegen::Scope;

use rustdoc_types::{
    Crate, GenericArgs, Id, Impl, Item, ItemEnum, Path, Trait, Type, Visibility,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

type PathIndex = HashMap<Vec<String>, (Rc<Crate>, Id)>;

use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

fn get_nightly_toolchain_path() -> Option<PathBuf> {
    // Get the path to the nightly rustc binary
    let nightly_rustc_output = Command::new("rustup")
        .arg("which")
        .arg("rustc")
        .arg("--toolchain")
        .arg("nightly")
        .output()
        .ok()?;

    let nightly_rustc = String::from_utf8(nightly_rustc_output.stdout)
        .ok()?
        .trim()
        .to_string();

    // Infer the base path to the nightly toolchain
    let nightly_toolchain_path = std::path::Path::new(&nightly_rustc)
        .parent()
        .and_then(|p| p.parent())?;

    Some(nightly_toolchain_path.to_path_buf())
}

fn get_json_toolchain_doc_path(toolchain_path: &PathBuf, component: &str) -> String {
    let json_path = toolchain_path
        .clone()
        .join(format!("share/doc/rust/json/{}.json", component));

    json_path.to_str().unwrap().to_owned()
}

fn index_crate(pi: &mut PathIndex, krate: Crate) {
    let krate = Rc::new(krate);
    krate
        .paths
        .iter()
        .filter(|(_, summary)| summary.crate_id == 0)
        .filter_map(|(id, summary)| {
            let item = krate.index.get(id)?;
            match &item.inner {
                ItemEnum::Struct(_) | ItemEnum::Enum(_) | ItemEnum::Trait(_) => {
                    if item.visibility == Visibility::Public {
                        Some((id, summary))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .for_each(|(id, summary)| {
            println!(" + {}", summary.path.join("::"));
            match pi.insert(summary.path.clone(), (krate.clone(), id.clone())) {
                Some((_other_krate, _other_id)) => {
                    panic!("conflict in path index for {}", summary.path.join("::"))
                }
                None => (),
            }
        })
}

fn index_rustdoc_json(
    pi: &mut PathIndex,
    json_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(json_path)?;
    let reader = BufReader::new(file);
    let krate: Crate = serde_json::from_reader(reader)?;
    index_crate(pi, krate);
    Ok(())
}

fn resolve_item(item: &Item, krate: &Crate) -> String {
    if let Some(p) = krate.paths.get(&item.id) {
        let full_path = p.path.join("::");
        full_path
    } else {
        match &item.name {
            Some(name) => name.clone(),
            None => "".to_owned(),
        }
    }
}

fn resolve_path(path: &Path, krate: &Crate) -> String {
    if let Some(p) = krate.paths.get(&path.id) {
        let mut full_path = p.path.join("::");
        if let Some(args) = &path.args {
            match args.as_ref() {
                GenericArgs::AngleBracketed { args, bindings } => {
                    let args_str = args
                        .iter()
                        .map(|arg| format!("{:?}", arg))
                        .collect::<Vec<_>>()
                        .join(", ");
                    full_path.push_str(&format!("<{}>", args_str));
                    if bindings.len() > 0 {
                        full_path.push_str(&format!(", bindings: {:?}", bindings));
                    }
                }
                GenericArgs::Parenthesized { inputs, output } => {
                    let inputs_str = inputs
                        .iter()
                        .map(|input| format!("{:?}", input))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if let Some(output) = output {
                        full_path
                            .push_str(&format!("Fn({}) -> {:?}", inputs_str, output));
                    } else {
                        full_path.push_str(&format!("Fn({})", inputs_str));
                    }
                }
            }
        }
        full_path
    } else {
        path.name.clone()
    }
}

fn lookup_item<'a>(pi: &'a PathIndex, path: &Path, krate: &Crate) -> Option<&'a Item> {
    let p = krate.paths.get(&path.id)?;
    let (krate, id) = pi.get(&p.path)?;
    krate.index.get(id)
}

fn find_trait<'a>(pi: &'a PathIndex, path: &Path, krate: &Crate) -> Option<&'a Trait> {
    let item = lookup_item(pi, path, krate)?;
    match &item.inner {
        ItemEnum::Trait(trait_) => Some(trait_),
        _ => None,
    }
}

fn print_trait(trait_: &Trait) -> String {
    let Trait {
        is_auto,
        is_unsafe,
        is_object_safe,
        items: _,
        generics: _,
        bounds: _,
        implementations: _,
    } = trait_;
    let txt = format!(
        "auto: {}, unsafe: {}, object_safe: {}",
        is_auto, is_unsafe, is_object_safe
    );
    // let txt = if generics.params.len() > 0 || generics.where_predicates.len() > 0 {
    //     txt + format!(", generics: {:?}", generics).as_str()
    // } else {
    //     txt
    // };
    // let txt = if bounds.len() > 0 {
    //     txt + format!(", bounds: {:?}", bounds).as_str()
    // } else {
    //     txt
    // };
    txt
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = "test/Cargo.toml";
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path)
        .exec()?;
    let resolve = &metadata.resolve.unwrap();
    let root_package_id = resolve.root.clone().unwrap();
    let root_resolve = resolve
        .nodes
        .iter()
        .find(|node| node.id == root_package_id)
        .unwrap();
    let dependencies: Vec<_> = root_resolve
        .dependencies
        .iter()
        .map(|id| metadata.packages.iter().find(|package| package.id == *id))
        .filter_map(|x| x)
        .collect();

    let nightly_toolchain_path = get_nightly_toolchain_path().unwrap();

    let mut path_index = PathIndex::new();

    //run the following to get those magic .json for core/std:
    // ```rustup component add rust-docs-json --toolchain nightly```

    index_rustdoc_json(
        &mut path_index,
        get_json_toolchain_doc_path(&nightly_toolchain_path, "core").as_str(),
    )?;

    index_rustdoc_json(
        &mut path_index,
        get_json_toolchain_doc_path(&nightly_toolchain_path, "std").as_str(),
    )?;

    index_rustdoc_json(
        &mut path_index,
        get_json_toolchain_doc_path(&nightly_toolchain_path, "alloc").as_str(),
    )?;

    let json_path = rustdoc_json::Builder::default()
        .toolchain("nightly")
        .manifest_path(manifest_path)
        .build()
        .unwrap();

    println!("Built and wrote rustdoc JSON to {:?}", &json_path);

    let file = File::open(json_path)?;
    let reader = BufReader::new(file);
    let krate: Crate = serde_json::from_reader(reader)?;
    index_crate(&mut path_index, krate.clone());

    dependencies.iter().for_each(|package| {
        println!(
            "Building rustdoc JSON for dependency package {} {} ...",
            package.name,
            package.version.to_string()
        );
        let maybe_json_path = rustdoc_json::Builder::default()
            .toolchain("nightly")
            .silent(true)
            .manifest_path(package.manifest_path.clone())
            .build();
        match maybe_json_path {
            Ok(json_path) => {
                println!("Built and wrote rustdoc JSON to {:?}", &json_path);
                index_rustdoc_json(&mut path_index, json_path.to_str().unwrap()).unwrap();
            }
            Err(error) => eprintln!(
                "Failed to build rustdoc JSON for {} {}: {}",
                package.name,
                package.version.to_string(),
                error
            ),
        }
    });

    // Collect implementations for easy lookup
    let impls: Vec<&Impl> = krate
        .index
        .values()
        .filter_map(|item| {
            if let ItemEnum::Impl(impl_item) = &item.inner {
                Some(impl_item)
            } else {
                None
            }
        })
        .collect();
    let mut scope = Scope::new();
    scope.import("ctor", "ctor");
    scope.import("ocaml_rs_smartptr", "register_trait");
    scope.import("ocaml_rs_smartptr", "register_type");

    let ctor_fn = scope.new_fn("type_registration").attr("ctor");

    // Iterate over all items in the crate
    for item in krate.index.values() {
        match &item.inner {
            ItemEnum::Struct(_) | ItemEnum::Enum(_) => {
                if item.visibility == Visibility::Public {
                    let item_fq_name = resolve_item(item, &krate);
                    println!("Public item: {}", item_fq_name);
                    ctor_fn.line(format!("register_type!({});", item_fq_name));
                    let matching_impls: Vec<_> = impls
                        .iter()
                        .filter(|impl_| {
                            if let Type::ResolvedPath(path) = &impl_.for_ {
                                path.id == item.id
                            } else {
                                false
                            }
                        })
                        .collect();
                    let mut ordinary_impls: Vec<(String, Option<Trait>)> = Vec::new();
                    let mut synthetic_impls: Vec<(String, Option<Trait>)> = Vec::new();
                    let mut blanket_impls: Vec<(String, Type, Option<Trait>)> =
                        Vec::new();
                    // Find and print the implemented traits
                    for impl_item in matching_impls {
                        if let Some(trait_ref) = &impl_item.trait_ {
                            let resolved = resolve_path(trait_ref, &krate);
                            let maybe_trait = find_trait(&path_index, trait_ref, &krate);
                            if impl_item.synthetic {
                                synthetic_impls.push((resolved, maybe_trait.cloned()))
                            } else if let Some(type_) = &impl_item.blanket_impl {
                                blanket_impls.push((
                                    resolved,
                                    type_.clone(),
                                    maybe_trait.cloned(),
                                ))
                            } else {
                                ordinary_impls.push((resolved, maybe_trait.cloned()))
                            }
                        }
                    }
                    println!("  Implements ordinary traits",);
                    for (item, maybe_trait) in ordinary_impls {
                        if let Some(trait_) = maybe_trait {
                            println!("    - {}, trait: {}", item, print_trait(&trait_))
                        } else {
                            println!("    - {}", item);
                        }
                    }
                    println!("  Implements synthetic traits",);
                    for (item, maybe_trait) in synthetic_impls {
                        if let Some(trait_) = maybe_trait {
                            println!("    - {}, trait: {}", item, print_trait(&trait_))
                        } else {
                            println!("    - {}", item);
                        }
                    }
                    println!("  Implements blanket traits",);
                    for (item, type_, maybe_trait) in blanket_impls {
                        if let Some(trait_) = maybe_trait {
                            println!(
                                "    - {} for {:?}, trait: {}",
                                item,
                                type_,
                                print_trait(&trait_)
                            )
                        } else {
                            println!("    - {} for {:?}", item, type_);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    println!("\nCode:\n");
    println!("{}", scope.to_string());

    Ok(())
}
