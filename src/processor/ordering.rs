use super::ItemSegment;
use crate::config::{ItemOrder, MoveFeature, MoveSelection, NormalizeConfig};
use syn::{Attribute, Item, ItemImpl, Type};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MoveAction {
    ModsMacros,
    ConstantsTypes,
    StructsEnumsImpls,
    TraitsForeignFunctions,
}

pub(super) fn reorder_items(
    items: Vec<ItemSegment>,
    config: &NormalizeConfig,
    selection: &MoveSelection,
) -> Vec<ItemSegment> {
    if selection.all {
        return reorder_all_items(items, config);
    }

    if selection.features.is_empty() {
        return items;
    }

    let mut out = items;
    for action in unique_actions(&selection.features) {
        out = apply_action(out, action, config);
    }
    out
}

fn action_for_feature(feature: MoveFeature) -> Option<MoveAction> {
    match feature {
        MoveFeature::Mods | MoveFeature::Macros => Some(MoveAction::ModsMacros),
        MoveFeature::Constants | MoveFeature::Types => Some(MoveAction::ConstantsTypes),
        MoveFeature::Structs | MoveFeature::Enums | MoveFeature::Impls => {
            Some(MoveAction::StructsEnumsImpls)
        }
        MoveFeature::Traits | MoveFeature::Foreign | MoveFeature::Functions => {
            Some(MoveAction::TraitsForeignFunctions)
        }
        MoveFeature::Attributes | MoveFeature::Imports | MoveFeature::Tests => None,
    }
}

fn unique_actions(features: &[MoveFeature]) -> Vec<MoveAction> {
    let mut unique = Vec::new();
    for feature in features {
        if let Some(action) = action_for_feature(*feature)
            && !unique.contains(&action) {
                unique.push(action);
            }
    }
    unique
}

fn apply_action(
    items: Vec<ItemSegment>,
    action: MoveAction,
    config: &NormalizeConfig,
) -> Vec<ItemSegment> {
    let mut selected = Vec::new();
    let mut selected_mask = Vec::with_capacity(items.len());

    for item in &items {
        let matches = item_matches_action(item, action);
        selected_mask.push(matches);
        if matches {
            selected.push(item.clone());
        }
    }

    let mut reordered_selected = reorder_action_items(selected, action, config).into_iter();
    let mut out = Vec::with_capacity(items.len());

    for (item, is_selected) in items.into_iter().zip(selected_mask) {
        if is_selected {
            if let Some(reordered) = reordered_selected.next() {
                out.push(reordered);
            } else {
                out.push(item);
            }
        } else {
            out.push(item);
        }
    }

    out
}

fn item_matches_action(segment: &ItemSegment, action: MoveAction) -> bool {
    match action {
        MoveAction::ModsMacros => match &segment.item {
            Item::Mod(item_mod) => !is_test_module(&item_mod.attrs, &item_mod.ident.to_string()),
            Item::Macro(_) => true,
            _ => false,
        },
        MoveAction::ConstantsTypes => {
            matches!(
                &segment.item,
                Item::Const(_) | Item::Static(_) | Item::Type(_)
            )
        }
        MoveAction::StructsEnumsImpls => {
            matches!(
                &segment.item,
                Item::Struct(_) | Item::Union(_) | Item::Enum(_) | Item::Impl(_)
            )
        }
        MoveAction::TraitsForeignFunctions => {
            matches!(
                &segment.item,
                Item::Trait(_) | Item::ForeignMod(_) | Item::Fn(_)
            )
        }
    }
}

fn reorder_action_items(
    items: Vec<ItemSegment>,
    action: MoveAction,
    config: &NormalizeConfig,
) -> Vec<ItemSegment> {
    match action {
        MoveAction::ModsMacros => reorder_mods_macros(items, config),
        MoveAction::ConstantsTypes => reorder_constants_types(items, config),
        MoveAction::StructsEnumsImpls => reorder_data_items(items, config),
        MoveAction::TraitsForeignFunctions => reorder_tail_items(items, config),
    }
}

fn reorder_all_items(items: Vec<ItemSegment>, config: &NormalizeConfig) -> Vec<ItemSegment> {
    let mut imports = Vec::new();
    let mut modules = Vec::new();
    let mut macros = Vec::new();
    let mut constants = Vec::new();
    let mut type_aliases = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut traits = Vec::new();
    let mut foreign = Vec::new();
    let mut functions = Vec::new();
    let mut tests = Vec::new();
    let mut others = Vec::new();
    let mut typed_impls: Vec<(String, Option<ItemSegment>)> = Vec::new();
    let mut fallback_impls = Vec::new();

    for item in items {
        match &item.item {
            Item::Use(_) => imports.push(item),
            Item::Mod(item_mod)
                if !is_test_module(&item_mod.attrs, &item_mod.ident.to_string()) =>
            {
                modules.push(item);
            }
            Item::Macro(_) => macros.push(item),
            Item::Const(_) | Item::Static(_) => constants.push(item),
            Item::Type(_) => type_aliases.push(item),
            Item::Struct(_) | Item::Union(_) => structs.push(item),
            Item::Enum(_) => enums.push(item),
            Item::Impl(item_impl) => {
                if let Some(type_name) = inherent_impl_target(item_impl) {
                    typed_impls.push((type_name, Some(item)));
                } else {
                    fallback_impls.push(item);
                }
            }
            Item::Trait(_) => traits.push(item),
            Item::ForeignMod(_) => foreign.push(item),
            Item::Fn(_) => functions.push(item),
            Item::Mod(item_mod) if is_test_module(&item_mod.attrs, &item_mod.ident.to_string()) => {
                tests.push(item);
            }
            _ => others.push(item),
        }
    }

    let mut out = Vec::new();
    out.extend(imports);

    if config.mods_before_macros() {
        out.extend(modules);
        out.extend(macros);
    } else {
        out.extend(macros);
        out.extend(modules);
    }

    if config.constants_before_types() {
        out.extend(constants);
        out.extend(type_aliases);
    } else {
        out.extend(type_aliases);
        out.extend(constants);
    }

    let structs_rank = config.rank(ItemOrder::Structs, 0);
    let enums_rank = config.rank(ItemOrder::Enums, 1);
    let impls_rank = config.rank(ItemOrder::Impls, 2);
    let attach_impls_to_structs = impls_rank <= enums_rank;
    let mut data_order = vec![
        (structs_rank, 0usize),
        (enums_rank, 1usize),
        (impls_rank, 2usize),
    ];
    data_order.sort_by_key(|(rank, tie_break)| (*rank, *tie_break));

    let mut structs_bucket = Some(structs);
    let mut enums_bucket = Some(enums);
    let mut fallback_impls_bucket = Some(fallback_impls);

    for (_, group) in data_order {
        match group {
            0 => {
                if let Some(struct_items) = structs_bucket.take() {
                    for item in struct_items {
                        let data_name = data_item_name(&item.item);
                        out.push(item);
                        if let (true, Some(data_name)) = (attach_impls_to_structs, data_name) {
                            for (impl_target, impl_item) in &mut typed_impls {
                                if impl_target == &data_name
                                    && let Some(segment) = impl_item.take() {
                                        out.push(segment);
                                    }
                            }
                        }
                    }
                }
            }
            1 => {
                if let Some(enum_items) = enums_bucket.take() {
                    out.extend(enum_items);
                }
            }
            2 => {
                for (_, impl_item) in &mut typed_impls {
                    if let Some(segment) = impl_item.take() {
                        out.push(segment);
                    }
                }
                if let Some(impl_items) = fallback_impls_bucket.take() {
                    out.extend(impl_items);
                }
            }
            _ => {}
        }
    }

    let mut tail_order = vec![
        (config.rank(ItemOrder::Traits, 0), 0usize),
        (config.rank(ItemOrder::Foreign, 1), 1usize),
        (config.rank(ItemOrder::Functions, 2), 2usize),
    ];
    tail_order.sort_by_key(|(rank, tie_break)| (*rank, *tie_break));
    for (_, group) in tail_order {
        match group {
            0 => out.extend(traits.iter().cloned()),
            1 => out.extend(foreign.iter().cloned()),
            2 => out.extend(functions.iter().cloned()),
            _ => {}
        }
    }

    out.extend(others);
    out.extend(tests);
    out
}

fn reorder_mods_macros(items: Vec<ItemSegment>, config: &NormalizeConfig) -> Vec<ItemSegment> {
    let mut modules = Vec::new();
    let mut macros = Vec::new();

    for item in items {
        match item.item {
            Item::Mod(_) => modules.push(item),
            Item::Macro(_) => macros.push(item),
            _ => {}
        }
    }

    if config.mods_before_macros() {
        modules.into_iter().chain(macros).collect()
    } else {
        macros.into_iter().chain(modules).collect()
    }
}

fn reorder_constants_types(items: Vec<ItemSegment>, config: &NormalizeConfig) -> Vec<ItemSegment> {
    let mut constants = Vec::new();
    let mut type_aliases = Vec::new();

    for item in items {
        match item.item {
            Item::Const(_) | Item::Static(_) => constants.push(item),
            Item::Type(_) => type_aliases.push(item),
            _ => {}
        }
    }

    if config.constants_before_types() {
        constants.into_iter().chain(type_aliases).collect()
    } else {
        type_aliases.into_iter().chain(constants).collect()
    }
}

fn reorder_data_items(items: Vec<ItemSegment>, config: &NormalizeConfig) -> Vec<ItemSegment> {
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut typed_impls: Vec<(String, Option<ItemSegment>)> = Vec::new();
    let mut fallback_impls = Vec::new();

    for item in items {
        match &item.item {
            Item::Struct(_) | Item::Union(_) => structs.push(item),
            Item::Enum(_) => enums.push(item),
            Item::Impl(item_impl) => {
                if let Some(type_name) = inherent_impl_target(item_impl) {
                    typed_impls.push((type_name, Some(item)));
                } else {
                    fallback_impls.push(item);
                }
            }
            _ => {}
        }
    }

    let structs_rank = config.rank(ItemOrder::Structs, 0);
    let enums_rank = config.rank(ItemOrder::Enums, 1);
    let impls_rank = config.rank(ItemOrder::Impls, 2);
    let attach_impls_to_structs = impls_rank <= enums_rank;
    let mut data_order = vec![
        (structs_rank, 0usize),
        (enums_rank, 1usize),
        (impls_rank, 2usize),
    ];
    data_order.sort_by_key(|(rank, tie_break)| (*rank, *tie_break));

    let mut out = Vec::new();
    let mut structs_bucket = Some(structs);
    let mut enums_bucket = Some(enums);
    let mut fallback_impls_bucket = Some(fallback_impls);

    for (_, group) in data_order {
        match group {
            0 => {
                if let Some(struct_items) = structs_bucket.take() {
                    for item in struct_items {
                        let data_name = data_item_name(&item.item);
                        out.push(item);
                        if let (true, Some(data_name)) = (attach_impls_to_structs, data_name) {
                            for (impl_target, impl_item) in &mut typed_impls {
                                if impl_target == &data_name
                                    && let Some(segment) = impl_item.take() {
                                        out.push(segment);
                                    }
                            }
                        }
                    }
                }
            }
            1 => {
                if let Some(enum_items) = enums_bucket.take() {
                    out.extend(enum_items);
                }
            }
            2 => {
                for (_, impl_item) in &mut typed_impls {
                    if let Some(segment) = impl_item.take() {
                        out.push(segment);
                    }
                }
                if let Some(impl_items) = fallback_impls_bucket.take() {
                    out.extend(impl_items);
                }
            }
            _ => {}
        }
    }

    out
}

fn reorder_tail_items(items: Vec<ItemSegment>, config: &NormalizeConfig) -> Vec<ItemSegment> {
    let mut traits = Vec::new();
    let mut foreign = Vec::new();
    let mut functions = Vec::new();

    for item in items {
        match item.item {
            Item::Trait(_) => traits.push(item),
            Item::ForeignMod(_) => foreign.push(item),
            Item::Fn(_) => functions.push(item),
            _ => {}
        }
    }

    let mut out = Vec::new();
    let mut tail_order = vec![
        (config.rank(ItemOrder::Traits, 0), 0usize),
        (config.rank(ItemOrder::Foreign, 1), 1usize),
        (config.rank(ItemOrder::Functions, 2), 2usize),
    ];
    tail_order.sort_by_key(|(rank, tie_break)| (*rank, *tie_break));
    for (_, group) in tail_order {
        match group {
            0 => out.extend(traits.iter().cloned()),
            1 => out.extend(foreign.iter().cloned()),
            2 => out.extend(functions.iter().cloned()),
            _ => {}
        }
    }

    out
}

fn data_item_name(item: &Item) -> Option<String> {
    match item {
        Item::Struct(item_struct) => Some(item_struct.ident.to_string()),
        Item::Enum(item_enum) => Some(item_enum.ident.to_string()),
        Item::Union(item_union) => Some(item_union.ident.to_string()),
        _ => None,
    }
}

fn inherent_impl_target(item_impl: &ItemImpl) -> Option<String> {
    if item_impl.trait_.is_some() {
        return None;
    }
    match item_impl.self_ty.as_ref() {
        Type::Path(type_path) if type_path.qself.is_none() => {
            let segment = type_path.path.segments.last()?;
            Some(segment.ident.to_string())
        }
        _ => None,
    }
}

fn is_test_module(attrs: &[Attribute], module_name: &str) -> bool {
    module_name == "tests" || attrs.iter().any(attr_is_cfg_test)
}

fn attr_is_cfg_test(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg") {
        return false;
    }
    let mut found = false;
    let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("test") {
            found = true;
        }
        Ok(())
    });
    found
}
