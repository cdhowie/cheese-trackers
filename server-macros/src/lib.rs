use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Field, Ident, ItemStruct, parse_macro_input, parse_str, spanned::Spanned};

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

    let mut diff_derives = vec![];
    let mut diff_serde = false;

    for attr in input
        .attrs
        .iter()
        .filter(|i| i.meta.path().is_ident("diff"))
    {
        attr.meta.require_list()?.parse_nested_meta(|m| {
            if m.path.is_ident("derive") {
                m.parse_nested_meta(|m| {
                    diff_derives.push(m.path);
                    Ok(())
                })?;
            } else if m.path.is_ident("serde") {
                if diff_serde {
                    return Err(m.error("duplicate serde attribute"));
                }

                diff_serde = true;
            } else {
                return Err(m.error("unsupported diff attribute"));
            }

            Ok(())
        })?;
    }

    if diff_serde {
        diff_derives.extend([
            parse_str("::serde::Serialize").unwrap(),
            parse_str("::serde::Deserialize").unwrap(),
        ]);
    }

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

    let output_doc = format!("A fieldwise diff of two [`{ident}`]s.");

    let output_struct = quote! {
        #[automatically_derived]
        #[doc = #output_doc]
        #[derive(#(#diff_derives),*)]
        #[derive(::serde::Serialize)]
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

#[proc_macro_derive(Model)]
pub fn derive_model(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    expand_derive_model(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(ModelWithAutoPrimaryKey, attributes(model))]
pub fn derive_model_with_auto_primary_key(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    expand_derive_model_with_auto_primary_key(input)
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

    let mut fields = vec![];

    for field in &input.fields {
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

    let model_columns = fields.iter().map(|f| {
        let variant = &f.iden;

        quote! { #iden_ident::#variant }
    });

    let model_into_values = fields.iter().map(|f| {
        let field = f.field.ident.as_ref().unwrap();

        quote! { self.#field.into() }
    });

    Ok(quote! {
        #[automatically_derived]
        impl crate::db::model::Model for #ident {
            type Iden = #iden_ident;

            fn table() -> Self::Iden {
                #iden_ident::Table
            }

            fn columns() -> &'static [Self::Iden] {
                &[
                    #( #model_columns ),*
                ]
            }

            fn into_values(self) -> impl Iterator<Item = ::sea_query::Value> {
                [
                    #( #model_into_values ),*
                ]
                .into_iter()
            }
        }
    })
}

fn expand_derive_model_with_auto_primary_key(
    input: ItemStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    struct ModelField<'a> {
        pub field: &'a Field,
        pub iden: Ident,
        pub is_primary_key: bool,
    }

    let ident = &input.ident;

    let iden_ident = format_ident!("{ident}Iden");

    let mut insertion_model_derives = vec![];

    for attr in input
        .attrs
        .iter()
        .filter(|i| i.meta.path().is_ident("model"))
    {
        attr.meta.require_list()?.parse_nested_meta(|m| {
            if m.path.is_ident("insertion_derive") {
                m.parse_nested_meta(|m| {
                    insertion_model_derives.push(m.path);
                    Ok(())
                })?;
            } else {
                return Err(m.error("unsupported model attribute"));
            }

            Ok(())
        })?;
    }

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
            is_primary_key,
        });
    }

    let primary_key = {
        let mut pkeys = fields.iter().filter(|f| f.is_primary_key);

        match (pkeys.next(), pkeys.next()) {
            (Some(f), None) => f,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "exactly one field must be tagged model(primary_key)",
                ));
            }
        }
    };

    let primary_key_type = &primary_key.field.ty;
    let primary_key_ident = primary_key.field.ident.as_ref().unwrap();

    let primary_key_iden = Ident::new(
        &primary_key_ident.to_string().to_case(Case::Pascal),
        primary_key_ident.span(),
    );

    let insertion_model_ident = format_ident!("{ident}Insertion");

    let insertion_model_fields = fields.iter().filter(|&f| !f.is_primary_key);

    let insertion_model_field_defs = insertion_model_fields.clone().map(|f| {
        let mut field = f.field.clone();
        field.attrs.clear();
        field
    });

    let insertion_model_from_model_fields = insertion_model_fields.clone().map(|f| {
        let ident = f.field.ident.as_ref().unwrap();

        quote! { #ident: value.#ident }
    });

    let insertion_model_from_model_fields_combine = insertion_model_fields.clone().map(|f| {
        let ident = f.field.ident.as_ref().unwrap();

        quote! { #ident: data.#ident }
    });

    let insertion_model_from_model_fields_split = insertion_model_fields.clone().map(|f| {
        let ident = f.field.ident.as_ref().unwrap();

        quote! { #ident: self.#ident }
    });

    let insertion_model_column_idens = insertion_model_fields.clone().map(|f| {
        let iden = &f.iden;

        quote! { #iden_ident::#iden }
    });

    let insertion_model_into_values = insertion_model_fields.map(|f| {
        let field = f.field.ident.as_ref().unwrap();

        quote! { value.#field.into() }
    });

    let insertion_model_doc = format!("Insertion model for [`{ident}`].");

    Ok(quote! {
        #[automatically_derived]
        #[allow(unused)]
        #[doc = #insertion_model_doc]
        #[derive(#(#insertion_model_derives),*)]
        pub struct #insertion_model_ident {
            #( #insertion_model_field_defs ),*
        }

        #[automatically_derived]
        impl crate::db::model::ModelWithAutoPrimaryKey for #ident {
            type InsertionModel = #insertion_model_ident;
            type PrimaryKey = #primary_key_type;

            fn insertion_columns() -> &'static [Self::Iden] {
                &[
                    #( #insertion_model_column_idens ),*
                ]
            }

            fn into_insertion_values(value: Self::InsertionModel) -> impl Iterator<Item = Value> {
                [
                    #( #insertion_model_into_values ),*
                ]
                .into_iter()
            }

            fn primary_key() -> Self::Iden {
                #iden_ident::#primary_key_iden
            }

            fn primary_key_value(&self) -> &Self::PrimaryKey {
                &self.#primary_key_ident
            }

            fn split_primary_key(self) -> (Self::PrimaryKey, Self::InsertionModel) {
                (
                    self.#primary_key_ident,
                    #insertion_model_ident {
                        #( #insertion_model_from_model_fields_split ),*
                    },
                )
            }

            fn combine_primary_key(key: Self::PrimaryKey, data: Self::InsertionModel) -> Self {
                Self {
                    #primary_key_ident: key,
                    #( #insertion_model_from_model_fields_combine ),*
                }
            }
        }

        #[automatically_derived]
        impl From<#ident> for #insertion_model_ident {
            fn from(value: #ident) -> Self {
                Self {
                    #( #insertion_model_from_model_fields ),*
                }
            }
        }
    })
}
