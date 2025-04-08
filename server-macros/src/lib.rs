use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, Ident, ItemStruct, parse_macro_input, spanned::Spanned};

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

#[proc_macro_derive(Model, attributes(model))]
pub fn derive_model(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    expand_derive_model(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_derive_model(input: ItemStruct) -> syn::Result<proc_macro2::TokenStream> {
    struct ModelField<'a> {
        pub field: &'a Field,
        pub iden: Ident,
    }

    let ident = &input.ident;

    let iden_ident = format_ident!("{ident}Iden");

    let mut primary_keys = vec![];
    let mut fields = vec![];

    for field in &input.fields {
        let mut is_primary_key = false;

        for attr in &field.attrs {
            if attr.meta.path().is_ident("model") {
                attr.meta.require_list()?.parse_nested_meta(|meta| {
                    if meta.path.is_ident("primary_key") {
                        if is_primary_key {
                            return Err(meta.error("duplicate primary_key"));
                        }

                        is_primary_key = true;
                    } else {
                        return Err(meta.error("unsupported model attribute"));
                    }

                    Ok(())
                })?;
            }
        }

        if is_primary_key {
            primary_keys.push(field.clone());
        }

        fields.push(ModelField {
            iden: Ident::new(
                &field
                    .ident
                    .as_ref()
                    .unwrap()
                    .to_string()
                    .to_case(Case::Pascal),
                field.ident.span(),
            ),

            field,
        });
    }

    // TODO: Support composite keys.
    if primary_keys.len() != 1 {
        return Err(syn::Error::new_spanned(
            input,
            "exactly one field must be tagged model(primary_key)",
        ));
    }

    let primary_key = primary_keys.into_iter().next().unwrap();

    let primary_key_type = &primary_key.ty;
    let primary_key_ident = primary_key.ident.as_ref().unwrap();

    let primary_key_iden = Ident::new(
        &primary_key_ident.to_string().to_case(Case::Pascal),
        primary_key_ident.span(),
    );

    let model_columns = fields.iter().map(|f| {
        let variant = &f.iden;

        quote! { #iden_ident::#variant }
    });

    let model_into_values = fields.iter().map(|f| {
        let field = f.field.ident.as_ref().unwrap();

        quote! { self.#field.into() }
    });

    let impl_model = quote! {
        impl crate::db::model::Model for #ident {
            type Iden = #iden_ident;
            type PrimaryKey = #primary_key_type;

            fn table() -> Self::Iden {
                #iden_ident::Table
            }

            fn columns() -> &'static [Self::Iden] {
                &[
                    #( #model_columns ),*
                ]
            }

            fn primary_key() -> Self::Iden {
                #iden_ident::#primary_key_iden
            }

            fn primary_key_value(&self) -> &Self::PrimaryKey {
                &self.#primary_key_ident
            }

            fn into_values(self) -> impl Iterator<Item = ::sea_query::Value> {
                [
                    #( #model_into_values ),*
                ]
                .into_iter()
            }
        }
    };

    Ok(quote! {
        #impl_model
    })
}
