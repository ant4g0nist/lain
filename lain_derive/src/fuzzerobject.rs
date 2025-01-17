use proc_macro2::TokenStream;

use quote::{quote, quote_spanned};

use crate::utils::*;
use syn::spanned::Spanned;
use syn::{Data, Ident};

use std::str::FromStr;

pub(crate) fn get_post_mutation_impl(ident: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);

                if fields.is_empty() {
                    return TokenStream::new();
                }

                let mut base_tokens = quote_spanned! { ident.span() => };

                for field in fields {
                    let field_name = &field.field.ident;
                    let field_ty = &field.field.ty;
                    base_tokens.extend(quote_spanned! { field.field.span() =>
                        <#field_ty>::fixup(&mut self.#field_name, mutator);
                    });
                }

                return base_tokens;
            } else {
                panic!("struct contains unnamed fields");
            }
        }
        _ => TokenStream::new(),
    }
}

pub(crate) fn get_post_fuzzer_iteration_impls(ident: &Ident, data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);

                if fields.is_empty() {
                    return TokenStream::new();
                }

                let mut base_tokens = quote_spanned!(ident.span() => );

                for field in fields {
                    let field_name = &field.field.ident;
                    let field_type = &field.field.ty;
                    base_tokens.extend(quote_spanned! { field.field.span() =>
                        <#field_type>::on_success(&self.#field_name);
                    });
                }

                return base_tokens;
            } else {
                panic!("struct contains unnamed fields");
            }
        }
        _ => TokenStream::new(),
    }
}

pub(crate) fn gen_mutate_impl(ident: &Ident, data: &Data) -> TokenStream {
    let mutate_body: TokenStream;

    match *data {
        Data::Enum(ref data) => {
            let enum_ident = ident.to_string();

            let mut enum_has_simple_variants = false;
            let mut mutate_match_arms: Vec<TokenStream> = Vec::new();
            for variant in &data.variants {
                let variant_ident = TokenStream::from_str(&format!(
                    "{}::{}",
                    enum_ident,
                    variant.ident.to_string()
                ))
                .unwrap();

                match variant.fields {
                    syn::Fields::Unnamed(ref fields) => {
                        let mut parameters = TokenStream::new();
                        let mut mutate_call = TokenStream::new();

                        for (i, ref unnamed) in fields.unnamed.iter().enumerate() {
                            let field_ty = &unnamed.ty;
                            let identifier =
                                TokenStream::from_str(&format!("field_{}", i)).unwrap();

                            mutate_call.extend(quote_spanned! { unnamed.span() =>
                                <#field_ty>::mutate(#identifier, mutator, None);
                            });

                            parameters
                                .extend(quote_spanned! {unnamed.span() => ref mut #identifier,});
                        }

                        mutate_match_arms.push(quote! {
                            #variant_ident(#parameters) => {
                                #mutate_call
                            },
                        });
                    }
                    syn::Fields::Unit => {
                        enum_has_simple_variants = true;
                        break;
                    }
                    _ => panic!("unsupported enum variant type"),
                }
            }

            mutate_body = if enum_has_simple_variants {
                // TODO: This will keep any #[fuzzer(ignore)] or #[weight(N)] attributes...
                // which we probably don't want.
                quote_spanned! { ident.span() =>
                    *self = <#ident>::new_fuzzed(mutator, None);
                }
            } else {
                quote_spanned! { ident.span() =>
                    match *self {
                        #(#mutate_match_arms)*
                    }
                }
            };
        }
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);
                mutate_body = gen_struct_mutate_impl(&fields);
            } else {
                panic!("struct contains unnamed fields");
            }
        }
        Data::Union(ref _data) => {
            panic!("unions are unsupported. Please use an enum with typed variants instead");
        }
    }

    quote_spanned! { ident.span() =>
        #[allow(unused)]
        fn mutate<R: ::lain::rand::Rng>(&mut self, mutator: &mut ::lain::mutator::Mutator<R>, constraints: Option<&Constraints<u8>>) {
            #mutate_body

            if mutator.should_fixup() {
                self.fixup(mutator);
            }
        }
    }
}

fn gen_struct_mutate_impl(fields: &[FuzzerObjectStructField]) -> TokenStream {
    let mutation_parts: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let mut field_mutation_tokens = TokenStream::new();
            let ty = &f.field.ty;
            let ident = &f.field.ident;

            field_mutation_tokens.extend(quote! {
                // constraints should be relatively cheap to clone
                <#ty>::mutate(&mut self.#ident, mutator, constraints);
                // TODO: For later
                // if let Some(ref mut constraints) = constraints {
                //     constraints.max_size -= self.ident.serialized_size();
                // }

                if mutator.should_early_bail_mutation() {
                    if mutator.should_fixup() {
                        <#ty>::fixup(&mut self.#ident, mutator);
                    }

                    return;
                }
            });

            field_mutation_tokens
        })
        .collect();

    quote! {
        #(#mutation_parts)*
    }
}
