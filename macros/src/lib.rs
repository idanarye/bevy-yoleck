use proc_macro2::TokenStream;

use quote::quote;
use syn::{Data, DeriveInput, Error, Field, Fields, LitStr, Token, Type};

#[proc_macro_derive(YoleckComponent)]
pub fn derive_yoleck_component(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match impl_yoleck_component_derive(input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_yoleck_component_derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = input.ident;
    let key = name.to_string();
    let result = quote!(
        impl YoleckComponent for #name {
            const KEY: &'static str = #key;
        }
    );
    Ok(result)
}

#[derive(Default, Debug)]
struct YoleckFieldAttrs {
    range: Option<(f64, f64)>,
    step: Option<f64>,
    label: Option<String>,
    tooltip: Option<String>,
    readonly: bool,
    hidden: bool,
    multiline: bool,
    color_picker: bool,
    asset_extensions: Option<Vec<String>>,
    entity_filter: Option<String>,
    speed: Option<f64>,
}

fn parse_number(expr: &syn::Expr) -> syn::Result<f64> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(i),
            ..
        }) => Ok(i.base10_parse::<f64>()?),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Float(f),
            ..
        }) => Ok(f.base10_parse::<f64>()?),
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr: inner,
            ..
        }) => Ok(-parse_number(inner)?),
        _ => Err(syn::Error::new_spanned(expr, "Expected numeric literal")),
    }
}

fn parse_field_attrs(field: &Field) -> Result<YoleckFieldAttrs, Error> {
    let mut attrs = YoleckFieldAttrs::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("yoleck") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("readonly") {
                attrs.readonly = true;
                return Ok(());
            }
            if meta.path.is_ident("hidden") {
                attrs.hidden = true;
                return Ok(());
            }
            if meta.path.is_ident("multiline") {
                attrs.multiline = true;
                return Ok(());
            }
            if meta.path.is_ident("color_picker") {
                attrs.color_picker = true;
                return Ok(());
            }

            if meta.path.is_ident("label") {
                let value: syn::LitStr = meta.value()?.parse()?;
                attrs.label = Some(value.value());
                return Ok(());
            }
            if meta.path.is_ident("tooltip") {
                let value: syn::LitStr = meta.value()?.parse()?;
                attrs.tooltip = Some(value.value());
                return Ok(());
            }
            if meta.path.is_ident("step") {
                let value: syn::LitFloat = meta.value()?.parse()?;
                attrs.step = Some(value.base10_parse()?);
                return Ok(());
            }
            if meta.path.is_ident("speed") {
                let value: syn::LitFloat = meta.value()?.parse()?;
                attrs.speed = Some(value.base10_parse()?);
                return Ok(());
            }
            if meta.path.is_ident("asset") {
                let value: syn::LitStr = meta.value()?.parse()?;
                attrs.asset_extensions = Some(
                    value
                        .value()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .collect(),
                );
                return Ok(());
            }
            if meta.path.is_ident("entity_ref") {
                let value: syn::LitStr = meta.value()?.parse()?;
                attrs.entity_filter = Some(value.value());
                return Ok(());
            }
            if meta.path.is_ident("range") {
                let content;
                syn::parenthesized!(content in meta.input);

                let expr: syn::Expr = content.parse()?;
                match expr {
                    syn::Expr::Range(syn::ExprRange {
                        start: Some(start),
                        end: Some(end),
                        limits: syn::RangeLimits::Closed(_),
                        ..
                    }) => {
                        let start_val = parse_number(&start)?;
                        let end_val = parse_number(&end)?;
                        attrs.range = Some((start_val, end_val));
                        return Ok(());
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            expr,
                            "Expected closed numeric range, e.g., `0.5..=10.0`",
                        ));
                    }
                }
            }

            Err(meta.error("unknown yoleck attribute"))
        })?;
    }

    Ok(attrs)
}

fn get_type_name(ty: &Type) -> String {
    match ty {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn quote_option<T, F>(opt: &Option<T>, f: F) -> TokenStream
where
    F: FnOnce(&T) -> TokenStream,
{
    match opt {
        Some(value) => {
            let inner = f(value);
            quote! { Some(#inner) }
        }
        None => quote! { None },
    }
}

fn generate_field_ui(field: &Field, attrs: &YoleckFieldAttrs) -> TokenStream {
    let field_name = field.ident.as_ref().unwrap();
    let field_name_str = attrs
        .label
        .clone()
        .unwrap_or_else(|| field_name.to_string().replace('_', " "));

    let range = quote_option(&attrs.range, |(min, max)| quote! { (#min, #max) });
    let speed = quote_option(&attrs.speed, |s| quote! { #s });
    let label_opt = quote_option(&attrs.label, |l| quote! { #l.to_string() });
    let tooltip = quote_option(&attrs.tooltip, |t| quote! { #t.to_string() });
    let entity_filter = quote_option(&attrs.entity_filter, |f| quote! { #f.to_string() });

    let readonly = attrs.readonly;
    let multiline = attrs.multiline;

    quote! {
        {
            use bevy_yoleck::auto_edit::{YoleckAutoEdit, FieldAttrs};
            let attrs = FieldAttrs {
                label: #label_opt,
                tooltip: #tooltip,
                range: #range,
                speed: #speed,
                readonly: #readonly,
                multiline: #multiline,
                entity_filter: #entity_filter,
            };
            YoleckAutoEdit::auto_edit_with_label_and_attrs(
                &mut value.#field_name,
                ui,
                #field_name_str,
                &attrs,
            );
        }
    }
}

#[proc_macro_derive(YoleckAutoEdit, attributes(yoleck))]
pub fn derive_yoleck_auto_edit(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match impl_yoleck_auto_edit_derive(input) {
        Ok(output) => output.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn impl_yoleck_auto_edit_derive(input: DeriveInput) -> Result<TokenStream, Error> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = if let Data::Struct(data) = &input.data {
        if let Fields::Named(fields) = &data.fields {
            &fields.named
        } else {
            return Err(Error::new_spanned(
                &input,
                "YoleckAutoEdit only supports structs with named fields",
            ));
        }
    } else {
        return Err(Error::new_spanned(
            &input,
            "YoleckAutoEdit only supports structs",
        ));
    };

    let mut field_uis = Vec::new();
    for field in fields {
        let attrs = parse_field_attrs(field)?;
        if attrs.hidden {
            continue;
        }
        field_uis.push(generate_field_ui(field, &attrs));
    }

    let mut entity_ref_fields = Vec::new();
    let mut entity_ref_field_names = Vec::new();
    for field in fields {
        if let Some(info) = parse_entity_ref_attrs(field)? {
            entity_ref_fields.push(info);
            entity_ref_field_names.push(
                field
                    .ident
                    .as_ref()
                    .expect("fields are taken from a named struct variant"),
            );
        }
    }

    let fields_array: Vec<TokenStream> = entity_ref_fields
        .iter()
        .map(|info| {
            let field_ident = &info.field_ident;
            let field_ident_str = LitStr::new(&field_ident.to_string(), field_ident.span());
            let filter = match &info.filter {
                Some(f) => quote! { Some(#f) },
                None => quote! { None },
            };

            quote! { (#field_ident_str, #filter) }
        })
        .collect();

    let match_arms: Vec<TokenStream> = entity_ref_fields
        .iter()
        .map(|info| {
            let field_ident = &info.field_ident;
            let field_ident_str = LitStr::new(&field_ident.to_string(), field_ident.span());

            quote! {
                #field_ident_str => &mut self.#field_ident
            }
        })
        .collect();

    let fields_count = entity_ref_fields.len();

    let get_entity_ref_mut_body = if entity_ref_fields.is_empty() {
        quote! {
            panic!("No entity ref fields in {}", stringify!(#name))
        }
    } else {
        quote! {
            match field_name {
                #(#match_arms,)*
                _ => panic!("Unknown entity ref field: {}", field_name),
            }
        }
    };

    let result = quote! {
        impl #impl_generics bevy_yoleck::auto_edit::YoleckAutoEdit for #name #ty_generics #where_clause {
            fn auto_edit(value: &mut Self, ui: &mut bevy_yoleck::egui::Ui) {
                use bevy_yoleck::egui;
                #(#field_uis)*
            }
        }

        impl #impl_generics bevy_yoleck::entity_ref::YoleckEntityRefAccessor for #name #ty_generics #where_clause {
            fn entity_ref_fields() -> &'static [(&'static str, Option<&'static str>)] {
                static FIELDS: [(&'static str, Option<&'static str>); #fields_count] = [
                    #(#fields_array),*
                ];
                &FIELDS
            }

            fn get_entity_ref_mut(&mut self, field_name: &str) -> &mut bevy_yoleck::entity_ref::YoleckEntityRef {
                #get_entity_ref_mut_body
            }

            fn resolve_entity_refs(&mut self, registry: &bevy_yoleck::prelude::YoleckUuidRegistry) {
                #(
                    let _ = self.#entity_ref_field_names.resolve(registry);
                )*
            }
        }
    };

    Ok(result)
}

#[derive(Debug)]
struct EntityRefFieldInfo {
    field_ident: syn::Ident,
    filter: Option<String>,
}

fn parse_entity_ref_attrs(field: &Field) -> Result<Option<EntityRefFieldInfo>, Error> {
    let type_name = get_type_name(&field.ty);

    if type_name != "YoleckEntityRef" {
        return Ok(None);
    }

    let field_ident = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "Expected named field"))?
        .clone();

    let mut info = EntityRefFieldInfo {
        field_ident,
        filter: None,
    };

    for attr in &field.attrs {
        if !attr.path().is_ident("yoleck") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("entity_ref") {
                if meta.input.peek(Token![=]) {
                    let value: syn::LitStr = meta.value()?.parse()?;
                    info.filter = Some(value.value());
                }
                return Ok(());
            }
            Ok(())
        })?;
    }

    Ok(Some(info))
}
