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

fn generate_field_ui(field: &Field, attrs: &YoleckFieldAttrs) -> TokenStream {
    let field_name = field.ident.as_ref().unwrap();
    let field_name_str = attrs
        .label
        .clone()
        .unwrap_or_else(|| field_name.to_string().replace('_', " "));

    let type_name = get_type_name(&field.ty);

    let tooltip_code = if let Some(tooltip) = &attrs.tooltip {
        quote! { .on_hover_text(#tooltip) }
    } else {
        quote! {}
    };

    let widget = match type_name.as_str() {
        "f32" | "f64" => {
            if let Some((min, max)) = attrs.range {
                let min = min as f32;
                let max = max as f32;
                quote! {
                    ui.horizontal(|ui| {
                        ui.label(#field_name_str);
                        ui.add(egui::Slider::new(&mut value.#field_name, #min..=#max))#tooltip_code;
                    });
                }
            } else {
                let speed = attrs.speed.unwrap_or(0.1) as f32;
                quote! {
                    ui.horizontal(|ui| {
                        ui.label(#field_name_str);
                        ui.add(egui::DragValue::new(&mut value.#field_name).speed(#speed))#tooltip_code;
                    });
                }
            }
        }
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => {
            if let Some((min, max)) = attrs.range {
                let min = min as i64;
                let max = max as i64;
                quote! {
                    ui.horizontal(|ui| {
                        ui.label(#field_name_str);
                        ui.add(egui::Slider::new(&mut value.#field_name, #min as _..=#max as _))#tooltip_code;
                    });
                }
            } else {
                let speed = attrs.speed.unwrap_or(1.0) as f32;
                quote! {
                    ui.horizontal(|ui| {
                        ui.label(#field_name_str);
                        ui.add(egui::DragValue::new(&mut value.#field_name).speed(#speed))#tooltip_code;
                    });
                }
            }
        }
        "bool" => {
            quote! {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut value.#field_name, #field_name_str)#tooltip_code;
                });
            }
        }
        "String" => {
            if attrs.multiline {
                quote! {
                    ui.label(#field_name_str);
                    ui.text_edit_multiline(&mut value.#field_name)#tooltip_code;
                }
            } else {
                quote! {
                    ui.horizontal(|ui| {
                        ui.label(#field_name_str);
                        ui.text_edit_singleline(&mut value.#field_name)#tooltip_code;
                    });
                }
            }
        }
        "Vec2" => {
            let speed = attrs.speed.unwrap_or(0.1) as f32;
            quote! {
                ui.horizontal(|ui| {
                    ui.label(#field_name_str);
                    ui.add(egui::DragValue::new(&mut value.#field_name.x).prefix("x: ").speed(#speed));
                    ui.add(egui::DragValue::new(&mut value.#field_name.y).prefix("y: ").speed(#speed));
                })#tooltip_code;
            }
        }
        "Vec3" => {
            let speed = attrs.speed.unwrap_or(0.1) as f32;
            quote! {
                ui.horizontal(|ui| {
                    ui.label(#field_name_str);
                    ui.add(egui::DragValue::new(&mut value.#field_name.x).prefix("x: ").speed(#speed));
                    ui.add(egui::DragValue::new(&mut value.#field_name.y).prefix("y: ").speed(#speed));
                    ui.add(egui::DragValue::new(&mut value.#field_name.z).prefix("z: ").speed(#speed));
                })#tooltip_code;
            }
        }
        "Vec4" => {
            let speed = attrs.speed.unwrap_or(0.1) as f32;
            quote! {
                ui.horizontal(|ui| {
                    ui.label(#field_name_str);
                    ui.add(egui::DragValue::new(&mut value.#field_name.x).prefix("x: ").speed(#speed));
                    ui.add(egui::DragValue::new(&mut value.#field_name.y).prefix("y: ").speed(#speed));
                    ui.add(egui::DragValue::new(&mut value.#field_name.z).prefix("z: ").speed(#speed));
                    ui.add(egui::DragValue::new(&mut value.#field_name.w).prefix("w: ").speed(#speed));
                })#tooltip_code;
            }
        }
        "Quat" => {
            let speed = attrs.speed.unwrap_or(1.0) as f32;
            quote! {
                ui.horizontal(|ui| {
                    ui.label(#field_name_str);
                    let (mut yaw, mut pitch, mut roll) = value.#field_name.to_euler(bevy::prelude::EulerRot::YXZ);
                    yaw = yaw.to_degrees();
                    pitch = pitch.to_degrees();
                    roll = roll.to_degrees();
                    let mut changed = false;
                    changed |= ui.add(egui::DragValue::new(&mut yaw).prefix("yaw: ").speed(#speed).suffix("°")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut pitch).prefix("pitch: ").speed(#speed).suffix("°")).changed();
                    changed |= ui.add(egui::DragValue::new(&mut roll).prefix("roll: ").speed(#speed).suffix("°")).changed();
                    if changed {
                        value.#field_name = bevy::prelude::Quat::from_euler(
                            bevy::prelude::EulerRot::YXZ,
                            yaw.to_radians(),
                            pitch.to_radians(),
                            roll.to_radians(),
                        );
                    }
                })#tooltip_code;
            }
        }
        "Color" | "Srgba" | "LinearRgba" => {
            quote! {
                ui.horizontal(|ui| {
                    ui.label(#field_name_str);
                    let srgba = value.#field_name.to_srgba();
                    let mut color_arr = [srgba.red, srgba.green, srgba.blue, srgba.alpha];
                    if ui.color_edit_button_rgba_unmultiplied(&mut color_arr).changed() {
                        value.#field_name = bevy::prelude::Color::srgba(color_arr[0], color_arr[1], color_arr[2], color_arr[3]);
                    }
                })#tooltip_code;
            }
        }
        "Option" => {
            quote! {
                ui.horizontal(|ui| {
                    ui.label(#field_name_str);
                    let mut has_value = value.#field_name.is_some();
                    if ui.checkbox(&mut has_value, "").changed() {
                        if has_value {
                            value.#field_name = Some(Default::default());
                        } else {
                            value.#field_name = None;
                        }
                    }
                    if let Some(ref mut inner) = value.#field_name {
                        bevy_yoleck::auto_edit::render_auto_edit_value(ui, inner);
                    }
                })#tooltip_code;
            }
        }
        "Vec" => {
            quote! {
                ui.collapsing(#field_name_str, |ui| {
                    let mut to_remove = None;
                    for (idx, item) in value.#field_name.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("[{}]", idx));
                            bevy_yoleck::auto_edit::render_auto_edit_value(ui, item);
                            if ui.small_button("−").clicked() {
                                to_remove = Some(idx);
                            }
                        });
                    }
                    if let Some(idx) = to_remove {
                        value.#field_name.remove(idx);
                    }
                    if ui.small_button("+").clicked() {
                        value.#field_name.push(Default::default());
                    }
                })#tooltip_code;
            }
        }
        _ => {
            quote! {
                ui.collapsing(#field_name_str, |ui| {
                    bevy_yoleck::auto_edit::render_auto_edit_value(ui, &mut value.#field_name);
                })#tooltip_code;
            }
        }
    };

    if attrs.readonly {
        quote! {
            ui.add_enabled_ui(false, |ui| {
                #widget
            });
        }
    } else {
        widget
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

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    &input,
                    "YoleckAutoEdit only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(Error::new_spanned(
                &input,
                "YoleckAutoEdit only supports structs",
            ))
        }
    };

    let mut field_uis = Vec::new();
    for field in fields {
        let attrs = parse_field_attrs(field)?;
        if attrs.hidden {
            continue;
        }
        let type_name = get_type_name(&field.ty);
        if type_name == "YoleckEntityRef" {
            continue;
        }
        field_uis.push(generate_field_ui(field, &attrs));
    }

    let mut entity_ref_fields = Vec::new();
    for field in fields {
        if let Some(info) = parse_entity_ref_attrs(field)? {
            entity_ref_fields.push(info);
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
