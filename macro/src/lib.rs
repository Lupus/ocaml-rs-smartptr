use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Path, PathSegment, Token, TypePath,
};

fn resolve_path(input_path: &Path, current_crate_name: &str) -> Path {
    if let Some(PathSegment { ref ident, .. }) = input_path.segments.first() {
        if ident == "crate" {
            let mut new_path = Path::from(format_ident!("{}", current_crate_name));
            new_path
                .segments
                .extend(input_path.segments.iter().skip(1).cloned());
            return new_path;
        }
    }

    let mut new_path = input_path.clone();
    new_path.leading_colon = Some(syn::Token![::](proc_macro2::Span::call_site()));
    new_path
}

fn globalize_path(input_path: &Path) -> Path {
    if let Some(PathSegment { ref ident, .. }) = input_path.segments.first() {
        if ident == "crate" {
            return input_path.clone();
        }
    }

    let mut new_path = input_path.clone();
    new_path.leading_colon = Some(syn::Token![::](proc_macro2::Span::call_site()));
    new_path
}

fn stringify_path(path: &Path) -> String {
    let mut path = path.clone();
    path.leading_colon = None;
    let token_stream = quote! { #path };
    token_stream.to_string().replace(" :: ", "::")
}

// This function contains the core logic and can be reused in tests
fn generate_type_registration(
    ty: &TypePath,
    marker_traits: &[Path],
    object_safe_traits: &[Path],
    current_crate_name: &str,
) -> proc_macro2::TokenStream {
    let mut ty = ty.clone();
    ty.path = globalize_path(&ty.path);
    let marker_traits: Vec<_> = marker_traits.iter().map(globalize_path).collect();
    let object_safe_traits: Vec<_> =
        object_safe_traits.iter().map(globalize_path).collect();
    let mut output = quote! {
        ocaml_rs_smartptr::registry::register_type::<#ty>();
    };
    let fq_name = stringify_path(&resolve_path(&ty.path, current_crate_name));
    let mut implementations = vec![];
    implementations.push(fq_name.clone());
    implementations.append(
        &mut marker_traits
            .iter()
            .map(|p| stringify_path(&resolve_path(p, current_crate_name)))
            .collect::<Vec<_>>(),
    );
    implementations.append(
        &mut object_safe_traits
            .iter()
            .map(|p| stringify_path(&resolve_path(p, current_crate_name)))
            .collect::<Vec<_>>(),
    );
    // Convert each LitStr into a TokenStream that represents a string literal in Rust
    let implementations: Vec<proc_macro2::TokenStream> = implementations
        .into_iter()
        .map(|value| {
            quote! { #value }
        })
        .collect();

    // Create the vector literal
    let implementations = quote! {
        vec![#(#implementations),*]
    };
    output.extend(quote! {
        ocaml_rs_smartptr::registry::register_type_info::<#ty>(#fq_name, #implementations);
    });

    output.extend(quote! {
        ocaml_rs_smartptr::registry::register::<#ty, #ty>(
            |x: &#ty| x as &#ty,
            |x: &mut #ty| x as &mut #ty
        );
    });

    for obj_trait in object_safe_traits {
        // Generate code for type -> obj_trait
        output.extend(quote! {
            ocaml_rs_smartptr::registry::register::<#ty, dyn #obj_trait>(
                |x: &#ty| x as &dyn #obj_trait,
                |x: &mut #ty| x as &mut dyn #obj_trait
            );
        });

        let combinations = marker_trait_combinations(&marker_traits);

        for (_, combination) in combinations {
            let full_trait = quote! { #obj_trait + #combination };

            output.extend(quote! {
                ocaml_rs_smartptr::registry::register::<#ty, dyn #full_trait>(
                    |x: &#ty| x as &(dyn #full_trait),
                    |x: &mut #ty| x as &mut (dyn #full_trait)
                );
            });
        }
    }

    output
}

// The procedural macro itself just handles parsing and calling the core logic
#[proc_macro]
pub fn register_type(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TypeRegisterInput);

    let output = generate_type_registration(
        &input.ty,
        &input.marker_traits,
        &input.object_safe_traits,
        &std::env::var("CARGO_CRATE_NAME").unwrap(),
    );
    output.into()
}

// Helper function to generate combinations of marker traits
fn marker_trait_combinations(
    marker_traits: &[Path],
) -> Vec<(Vec<&syn::Path>, proc_macro2::TokenStream)> {
    let mut combinations = vec![(vec![], quote! {})];

    for marker_trait in marker_traits {
        let mut new_combinations = Vec::new();

        for (combination_paths, combination_tokens) in &combinations {
            if combination_paths.is_empty() {
                // If the current combination is empty, just add the marker trait
                new_combinations.push((vec![marker_trait], quote! { #marker_trait }));
            } else {
                // Otherwise, add a `+` between the current combination and the new marker trait
                let mut combination_paths = combination_paths.clone();
                combination_paths.push(marker_trait);
                new_combinations.push((
                    combination_paths,
                    quote! { #combination_tokens + #marker_trait },
                ));
            }
        }

        combinations.extend(new_combinations);
    }

    combinations
}

struct TypeRegisterInput {
    ty: TypePath,
    marker_traits: Vec<Path>,
    object_safe_traits: Vec<Path>,
    #[allow(dead_code)]
    conversions: Vec<Conversion>,
}

struct Conversion {
    #[allow(dead_code)]
    kind: ConversionKind,
    #[allow(dead_code)]
    to_type: Path,
    #[allow(dead_code)]
    method: syn::ExprClosure,
}

#[derive(Debug)]
enum ConversionKind {
    Into,
    From,
    AsRef,
    AsMut,
}

impl Parse for Conversion {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let kind: ConversionKind = input.parse()?;
        input.parse::<Token![::]>()?;
        let to_type: Path = input.parse()?;
        input.parse::<Token![=>]>()?;
        let method: syn::ExprClosure = input.parse()?;

        Ok(Conversion {
            kind,
            to_type,
            method,
        })
    }
}

impl Parse for ConversionKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "Into" => Ok(ConversionKind::Into),
            "From" => Ok(ConversionKind::From),
            "AsRef" => Ok(ConversionKind::AsRef),
            "AsMut" => Ok(ConversionKind::AsMut),
            _ => Err(syn::Error::new(
                ident.span(),
                "Expected one of: Into, From, AsRef, AsMut",
            )),
        }
    }
}

impl Parse for TypeRegisterInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let _ = syn::braced!(content in input);

        let ty = parse_named_field(&content, "ty")?;
        let marker_traits = parse_named_list(&content, "marker_traits")?;
        let object_safe_traits = parse_named_list(&content, "object_safe_traits")?;
        let conversions = vec![];

        Ok(TypeRegisterInput {
            ty,
            marker_traits,
            object_safe_traits,
            conversions,
        })
    }
}

fn parse_named_field<T: Parse>(input: ParseStream, name: &str) -> syn::Result<T> {
    let ident: syn::Ident = input.parse()?;
    if ident == name {
        input.parse::<Token![:]>()?;
        let value: T = input.parse()?;
        input.parse::<Token![,]>().ok(); // Optional trailing comma
        Ok(value)
    } else {
        Err(syn::Error::new(
            ident.span(),
            format!("Expected '{}'", name),
        ))
    }
}

fn parse_named_list<T: Parse>(input: ParseStream, name: &str) -> syn::Result<Vec<T>> {
    let ident: syn::Ident = input.parse()?;
    if ident == name {
        input.parse::<Token![:]>()?;
        let content;
        let _ = syn::bracketed!(content in input);
        let values = Punctuated::<T, Token![,]>::parse_terminated(&content)?;
        input.parse::<Token![,]>().ok(); // Optional trailing comma
        Ok(values.into_iter().collect())
    } else {
        Err(syn::Error::new(
            ident.span(),
            format!("Expected '{}'", name),
        ))
    }
}

struct TraitRegisterInput {
    ty: TypePath,
    marker_traits: Vec<Path>,
}

impl Parse for TraitRegisterInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        let _ = syn::braced!(content in input);

        let ty = parse_named_field(&content, "ty")?;
        let marker_traits = parse_named_list(&content, "marker_traits")?;

        Ok(TraitRegisterInput { ty, marker_traits })
    }
}

// This function contains the core logic and can be reused in tests
fn generate_trait_registration(
    ty: &TypePath,
    marker_traits: &[Path],
    current_crate_name: &str,
) -> proc_macro2::TokenStream {
    let mut ty = ty.clone();
    ty.path = globalize_path(&ty.path);
    let marker_traits: Vec<_> = marker_traits.iter().map(globalize_path).collect();
    let mut output = quote! {
        ocaml_rs_smartptr::registry::register_type::<dyn #ty>();
    };
    let fq_name = stringify_path(&resolve_path(&ty.path, current_crate_name));
    let mut implementations = vec![];
    implementations.push(fq_name.clone());
    implementations.append(
        &mut marker_traits
            .iter()
            .map(|p| stringify_path(&resolve_path(p, current_crate_name)))
            .collect::<Vec<_>>(),
    );
    // Convert each LitStr into a TokenStream that represents a string literal in Rust
    let implementations: Vec<proc_macro2::TokenStream> = implementations
        .into_iter()
        .map(|value| {
            quote! { #value }
        })
        .collect();

    // Create the vector literal
    let implementations = quote! {
        vec![#(#implementations),*]
    };
    output.extend(quote! {
        ocaml_rs_smartptr::registry::register_type_info::<dyn #ty>(#fq_name, #implementations);
    });

    output.extend(quote! {
        ocaml_rs_smartptr::registry::register::<Box<dyn #ty>, dyn #ty>(
            |x: &Box<dyn #ty>| x.as_ref(),
            |x: &mut Box<dyn #ty>| x.as_mut()
        );
    });

    let combinations = marker_trait_combinations(&marker_traits);

    for (combination_paths, combination_tokens) in combinations {
        let full_trait = quote! { #ty + #combination_tokens };
        output.extend(quote! {
            ocaml_rs_smartptr::registry::register_type::<dyn #full_trait>();
        });
        let mut implementations = vec![];
        implementations.push(fq_name.clone());
        implementations.append(
            &mut combination_paths
                .iter()
                .map(|p| stringify_path(&resolve_path(p, current_crate_name)))
                .collect::<Vec<_>>(),
        );
        // Convert each LitStr into a TokenStream that represents a string literal in Rust
        let implementations: Vec<proc_macro2::TokenStream> = implementations
            .into_iter()
            .map(|value| {
                quote! { #value }
            })
            .collect();

        // Create the vector literal
        let implementations = quote! {
            vec![#(#implementations),*]
        };
        output.extend(quote! {
            ocaml_rs_smartptr::registry::register_type_info::<dyn #full_trait>(#fq_name, #implementations);
        });

        output.extend(quote! {
            ocaml_rs_smartptr::registry::register::<Box<dyn #full_trait>, dyn #full_trait>(
                |x: &Box<dyn #full_trait>| x.as_ref(),
                |x: &mut Box<dyn #full_trait>| x.as_mut()
            );
        });
    }

    output
}

// The procedural macro itself just handles parsing and calling the core logic
#[proc_macro]
pub fn register_trait(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as TraitRegisterInput);

    let output = generate_trait_registration(
        &input.ty,
        &input.marker_traits,
        &std::env::var("CARGO_CRATE_NAME").unwrap(),
    );
    output.into()
}

#[cfg(test)]
mod generation_tests {
    use super::*;

    use quote::quote;
    use syn::{parse_quote, Path, TypePath};

    fn pretty_print_item(ts: proc_macro2::TokenStream) -> String {
        let code = "fn main() {\n".to_owned() + &ts.to_string() + "\n}";
        let file = syn::parse_file(&code).unwrap();
        prettyplease::unparse(&file)
    }

    #[test]
    fn test_register_traits_macro_crate() {
        // Define the input to the core function
        let ty: TypePath = parse_quote! { crate::test_types::MyType };
        let marker_traits: Vec<Path> = vec![
            parse_quote! { crate::test_types::MyMarkerTrait1 },
            parse_quote! { crate::test_types::MyMarkerTrait2 },
        ];
        let object_safe_traits: Vec<Path> = vec![
            parse_quote! { crate::test_types::MyObjectSafeTrait1 },
            parse_quote! { crate::test_types::MyObjectSafeTrait2 },
        ];

        // Generate the actual output using the core logic function
        let output_tokens = generate_type_registration(
            &ty,
            &marker_traits,
            &object_safe_traits,
            "this_crate",
        );

        let expected_output = quote! {
            ocaml_rs_smartptr::registry::register_type::<crate::test_types::MyType>();
            ocaml_rs_smartptr::registry::register_type_info::<
                crate::test_types::MyType,
            >(
                "this_crate::test_types::MyType",
                vec![
                    "this_crate::test_types::MyType",
                    "this_crate::test_types::MyMarkerTrait1",
                    "this_crate::test_types::MyMarkerTrait2",
                    "this_crate::test_types::MyObjectSafeTrait1",
                    "this_crate::test_types::MyObjectSafeTrait2"
                ],
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                crate::test_types::MyType,
            >(
                |x: &crate::test_types::MyType| x as &crate::test_types::MyType,
                |x: &mut crate::test_types::MyType| x as &mut crate::test_types::MyType,
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait1,
            >(
                |x: &crate::test_types::MyType| x as &dyn crate::test_types::MyObjectSafeTrait1,
                |x: &mut crate::test_types::MyType| {
                    x as &mut dyn crate::test_types::MyObjectSafeTrait1
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait1,
            >(
                |x: &crate::test_types::MyType| {
                    x as &(dyn crate::test_types::MyObjectSafeTrait1)
                },
                |x: &mut crate::test_types::MyType| {
                    x as &mut (dyn crate::test_types::MyObjectSafeTrait1)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait1,
            >(
                |x: &crate::test_types::MyType| {
                    x
                        as &(dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait1)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait1)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait2,
            >(
                |x: &crate::test_types::MyType| {
                    x
                        as &(dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait2)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait2)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait1 + crate::test_types::MyMarkerTrait2,
            >(
                |x: &crate::test_types::MyType| {
                    x
                        as &(dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait1 + crate::test_types::MyMarkerTrait2)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn crate::test_types::MyObjectSafeTrait1 + crate::test_types::MyMarkerTrait1 + crate::test_types::MyMarkerTrait2)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait2,
            >(
                |x: &crate::test_types::MyType| x as &dyn crate::test_types::MyObjectSafeTrait2,
                |x: &mut crate::test_types::MyType| {
                    x as &mut dyn crate::test_types::MyObjectSafeTrait2
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait2,
            >(
                |x: &crate::test_types::MyType| {
                    x as &(dyn crate::test_types::MyObjectSafeTrait2)
                },
                |x: &mut crate::test_types::MyType| {
                    x as &mut (dyn crate::test_types::MyObjectSafeTrait2)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait1,
            >(
                |x: &crate::test_types::MyType| {
                    x
                        as &(dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait1)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait1)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait2,
            >(
                |x: &crate::test_types::MyType| {
                    x
                        as &(dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait2)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait2)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait1 + crate::test_types::MyMarkerTrait2,
            >(
                |x: &crate::test_types::MyType| {
                    x
                        as &(dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait1 + crate::test_types::MyMarkerTrait2)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn crate::test_types::MyObjectSafeTrait2 + crate::test_types::MyMarkerTrait1 + crate::test_types::MyMarkerTrait2)
                },
            );
        };

        // Use prettyplease to format the output and expected output
        let output = pretty_print_item(output_tokens);
        let expected_output = pretty_print_item(expected_output);

        // Assert that the output matches the expected output
        assert_eq!(output, expected_output);
    }

    #[test]
    fn test_register_traits_macro_global() {
        // Define the input to the core function
        let ty: TypePath = parse_quote! { crate::test_types::MyType };
        let marker_traits: Vec<Path> = vec![
            parse_quote! { core::marker::Send },
            parse_quote! { core::marker::Sync },
        ];
        let object_safe_traits: Vec<Path> = vec![parse_quote! { std::error::Error }];

        // Generate the actual output using the core logic function
        let output_tokens = generate_type_registration(
            &ty,
            &marker_traits,
            &object_safe_traits,
            "this_crate",
        );

        let expected_output = quote! {
            ocaml_rs_smartptr::registry::register_type::<crate::test_types::MyType>();
            ocaml_rs_smartptr::registry::register_type_info::<
                crate::test_types::MyType,
            >(
                "this_crate::test_types::MyType",
                vec![
                    "this_crate::test_types::MyType",
                    "core::marker::Send",
                    "core::marker::Sync",
                    "std::error::Error"
                ],
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                crate::test_types::MyType,
            >(
                |x: &crate::test_types::MyType| x as &crate::test_types::MyType,
                |x: &mut crate::test_types::MyType| x as &mut crate::test_types::MyType,
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn ::std::error::Error,
            >(
                |x: &crate::test_types::MyType| x as &dyn ::std::error::Error,
                |x: &mut crate::test_types::MyType| x as &mut dyn ::std::error::Error,
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn ::std::error::Error,
            >(
                |x: &crate::test_types::MyType| x as &(dyn ::std::error::Error),
                |x: &mut crate::test_types::MyType| x as &mut (dyn ::std::error::Error),
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn ::std::error::Error + ::core::marker::Send,
            >(
                |x: &crate::test_types::MyType| {
                    x as &(dyn ::std::error::Error + ::core::marker::Send)
                },
                |x: &mut crate::test_types::MyType| {
                    x as &mut (dyn ::std::error::Error + ::core::marker::Send)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn ::std::error::Error + ::core::marker::Sync,
            >(
                |x: &crate::test_types::MyType| {
                    x as &(dyn ::std::error::Error + ::core::marker::Sync)
                },
                |x: &mut crate::test_types::MyType| {
                    x as &mut (dyn ::std::error::Error + ::core::marker::Sync)
                },
            );
            ocaml_rs_smartptr::registry::register::<
                crate::test_types::MyType,
                dyn ::std::error::Error + ::core::marker::Send + ::core::marker::Sync,
            >(
                |x: &crate::test_types::MyType| {
                    x as &(dyn ::std::error::Error + ::core::marker::Send + ::core::marker::Sync)
                },
                |x: &mut crate::test_types::MyType| {
                    x
                        as &mut (dyn ::std::error::Error + ::core::marker::Send + ::core::marker::Sync)
                },
            );
        };

        // Use prettyplease to format the output and expected output
        let output = pretty_print_item(output_tokens);
        let expected_output = pretty_print_item(expected_output);

        // Assert that the output matches the expected output
        assert_eq!(output, expected_output);
    }
}

#[cfg(test)]
mod parsing_tests {
    use super::*;
    use quote::ToTokens;
    use syn::{parse_quote, TypePath};

    #[test]
    fn test_basic_parsing() {
        let input: TypeRegisterInput = syn::parse_quote! {
            {
                ty: crate::MyType,
                marker_traits: [crate::MyMarkerTrait1, crate::MyMarkerTrait2],
                object_safe_traits: [crate::MyObjectSafeTrait1, crate::MyObjectSafeTrait2],
            }
        };
        /*  TODO
                conversions: [
                    Into(crate::OtherType) => |x: crate::MyType| x.into(),
                    AsRef(crate::OtherType) => |x: &crate::MyType| x.as_ref(),
                    AsMut(crate::OtherType) => |x: &mut crate::MyType| x.as_mut()
                ],
        */

        let expected_ty: TypePath = parse_quote!(crate::MyType);
        assert_eq!(
            input.ty.to_token_stream().to_string(),
            expected_ty.to_token_stream().to_string()
        );
        assert_eq!(input.marker_traits.len(), 2);
        assert_eq!(input.object_safe_traits.len(), 2);
        assert!(input.conversions.is_empty());
        // assert_eq!(input.conversions.len(), TODO);

        // TODO Check one of the conversions
        // let first_conversion = &input.conversions[0];
        // if let ConversionKind::Into = first_conversion.kind {
        //     let expected_path: Path = parse_quote!(crate::OtherType);
        //     assert_eq!(
        //         first_conversion.to_type.to_token_stream().to_string(),
        //         expected_path.to_token_stream().to_string()
        //     );
        // } else {
        //     panic!("Expected ConversionKind::Into");
        // }
    }

    #[test]
    fn test_missing_optional_section() {
        let input: TypeRegisterInput = syn::parse_quote! {
            {
                ty: crate::MyType,
                marker_traits: [crate::MyMarkerTrait1, crate::MyMarkerTrait2],
                object_safe_traits: [crate::MyObjectSafeTrait1, crate::MyObjectSafeTrait2],
            }
        };

        let expected_ty: TypePath = parse_quote!(crate::MyType);
        assert_eq!(
            input.ty.to_token_stream().to_string(),
            expected_ty.to_token_stream().to_string()
        );
        assert_eq!(input.marker_traits.len(), 2);
        assert_eq!(input.object_safe_traits.len(), 2);
        assert!(input.conversions.is_empty());
    }

    #[test]
    fn test_empty_sections() {
        let input: TypeRegisterInput = syn::parse_quote! {
            {
                ty: crate::MyType,
                marker_traits: [],
                object_safe_traits: [],
            }
        };

        let expected_ty: TypePath = parse_quote!(crate::MyType);
        assert_eq!(
            input.ty.to_token_stream().to_string(),
            expected_ty.to_token_stream().to_string()
        );
        assert!(input.marker_traits.is_empty());
        assert!(input.object_safe_traits.is_empty());
        assert!(input.conversions.is_empty());
    }

    #[test]
    fn test_invalid_input_missing_type() {
        let result: syn::Result<TypeRegisterInput> = syn::parse_str(
            r#"
            {
                marker_traits: [crate::MyMarkerTrait1, crate::MyMarkerTrait2],
                object_safe_traits: [crate::MyObjectSafeTrait1, crate::MyObjectSafeTrait2],
            }
            "#,
        );

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Expected 'ty'"));
        }
    }

    #[test]
    fn test_invalid_input_wrong_keyword() {
        let result: syn::Result<TypeRegisterInput> = syn::parse_str(
            r#"
            {
                typo_type: crate::MyType,
                marker_traits: [crate::MyMarkerTrait1, crate::MyMarkerTrait2],
                object_safe_traits: [crate::MyObjectSafeTrait1, crate::MyObjectSafeTrait2],
            }
            "#,
        );

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Expected 'ty'"));
        }
    }
}
