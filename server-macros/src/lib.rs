use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ItemStruct, parse_macro_input};

#[proc_macro_derive(IntoFieldwiseDiff, attributes(diff))]
pub fn derive_fieldwise_diff(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    expand_derive_fieldwise_diff(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_derive_fieldwise_diff(input: ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    let ident = input.ident;

    let output_ident = format_ident!("{ident}FieldwiseDiff");

    let fields: Vec<_> = input
        .fields
        .into_iter()
        .filter_map(|f| {
            (|| {
                let mut skip = false;

                for attr in &f.attrs {
                    if attr.meta.path().is_ident("diff") {
                        attr.meta.require_list()?.parse_nested_meta(|meta| {
                            if meta.path.is_ident("skip") {
                                if skip {
                                    return Err(meta.error("duplicate skip"));
                                }

                                skip = true;
                            } else {
                                return Err(meta.error("unsupported diff attribute"));
                            }

                            Ok(())
                        })?;
                    }
                }

                Ok::<_, syn::Error>((!skip).then_some(f))
            })()
            .transpose()
        })
        .collect::<Result<_, _>>()?;

    let field_names: Vec<_> = fields
        .iter()
        .map(|f| f.ident.as_ref().unwrap().clone())
        .collect();

    let trait_impl = quote! {
        #[automatically_derived]
        impl crate::diff::IntoFieldwiseDiff<#ident> for #ident {
            type Output = #output_ident;

            fn into_fieldwise_diff(self, other: Self) -> Self::Output {
                #output_ident {
                    #(
                        #field_names: crate::diff::FieldDiff::new(
                            self.#field_names,
                            other.#field_names,
                        )
                    ),*
                }
            }
        }
        impl crate::diff::IntoFieldwiseDiff<&#ident> for &#ident {
            type Output = #output_ident;

            fn into_fieldwise_diff(self, other: &#ident) -> Self::Output {
                #output_ident {
                    #(
                        #field_names: crate::diff::FieldDiff::new_cloned(
                            &self.#field_names,
                            &other.#field_names,
                        )
                    ),*
                }
            }
        }
    };

    let isempty_impl = quote! {
        #[automatically_derived]
        impl crate::diff::IsEmpty for #output_ident {
            fn is_empty(&self) -> bool {
                true #(
                    && ::std::option::Option::is_none(&self.#field_names)
                )*
            }
        }
    };

    let output_fields = fields.into_iter().map(|f| {
        let name = f.ident.unwrap();
        let ty = f.ty;

        quote! { pub #name: ::std::option::Option<crate::diff::FieldDiff<#ty>> }
    });

    let output_doc = format!("A fieldwise diff of two `{ident}`s.");

    let output_struct = quote! {
        #[automatically_derived]
        #[doc = #output_doc]
        #[derive(Debug, Default, Clone, ::serde::Serialize)]
        pub struct #output_ident {
            #(
                #[serde(skip_serializing_if = "Option::is_none")]
                #output_fields
            ),*
        }
    };

    Ok(quote! {
        #output_struct

        #trait_impl

        #isempty_impl
    })
}
