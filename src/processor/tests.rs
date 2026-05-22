use super::model::{ItemSegment, NormalizedFile, Normalizer};
use super::ordering::reorder_items;
use super::render::render_segments;
use super::text::{
    differs_only_by_whitespace, normalize_function_spacing, promote_leading_item_comments,
    restore_promoted_comment_style,
};
use crate::config::{ItemOrder, MoveFeature, MoveSelection, NormalizeConfig};

fn selection_all() -> MoveSelection {
    MoveSelection {
        all: true,
        features: Vec::new(),
    }
}

fn parse_item(src: &str) -> syn::Item {
    syn::parse_str(src).expect("valid Rust item")
}

fn segment(src: &str) -> ItemSegment {
    ItemSegment {
        item: parse_item(src),
        leading_comments: Vec::new(),
        module_doc_comments: Vec::new(),
        source: src.to_owned(),
    }
}

#[test]
fn puts_mod_before_macros_by_default() {
    let items = vec![
        segment("macro_rules! m { () => {}; }"),
        segment("mod attacks;"),
    ];
    let reordered = reorder_items(items, &NormalizeConfig::default(), &selection_all());
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &NormalizeConfig::default(),
        None,
    );
    let mod_pos = rendered.find("mod attacks;").expect("mod item present");
    let macro_pos = rendered.find("macro_rules! m").expect("macro item present");
    assert!(mod_pos < macro_pos);
}

#[test]
fn puts_macros_after_mods_and_use_items_by_default() {
    let items = vec![
        segment("mod attacks;"),
        segment("include!(\"generated.rs\");"),
        segment("use crate::types::Move;"),
    ];
    let reordered = reorder_items(items, &NormalizeConfig::default(), &selection_all());
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &NormalizeConfig::default(),
        None,
    );
    assert!(
        rendered
            .starts_with("use crate::types::Move;\n\nmod attacks;\n\ninclude!(\"generated.rs\");"),
    );
}

#[test]
fn can_put_macros_before_mods_via_config() {
    let items = vec![
        segment("macro_rules! m { () => {}; }"),
        segment("mod attacks;"),
    ];
    let config = NormalizeConfig {
        order: vec![
            ItemOrder::Imports,
            ItemOrder::Macros,
            ItemOrder::Mods,
            ItemOrder::Constants,
            ItemOrder::Types,
            ItemOrder::Enums,
            ItemOrder::Structs,
            ItemOrder::Impls,
            ItemOrder::Traits,
            ItemOrder::Foreign,
            ItemOrder::Functions,
            ItemOrder::Tests,
        ],
        ..NormalizeConfig::default()
    };
    let reordered = reorder_items(items, &config, &selection_all());
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &config,
        None,
    );
    let mod_pos = rendered.find("mod attacks;").expect("mod item present");
    let macro_pos = rendered.find("macro_rules! m").expect("macro item present");
    assert!(macro_pos < mod_pos);
}

#[test]
fn puts_constants_before_type_aliases_by_default() {
    let items = vec![
        segment("type Score = i32;"),
        segment("const DEFAULT: Score = 0;"),
    ];
    let reordered = reorder_items(items, &NormalizeConfig::default(), &selection_all());
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &NormalizeConfig::default(),
        None,
    );
    assert!(rendered.starts_with("const DEFAULT: Score = 0;\n\ntype Score = i32;"));
}

#[test]
fn puts_ffi_before_free_functions_by_default() {
    let items = vec![
        segment("fn eval() -> Score { Score(0) }"),
        segment("extern \"C\" {\n    fn eval_native() -> i32;\n}"),
    ];
    let reordered = reorder_items(items, &NormalizeConfig::default(), &selection_all());
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &NormalizeConfig::default(),
        None,
    );
    assert!(rendered.starts_with(
        "extern \"C\" {\n    fn eval_native() -> i32;\n}\n\nfn eval() -> Score { Score(0) }"
    ));
}

#[test]
fn honors_struct_impl_enum_priority_from_config() {
    let items = vec![
        segment("enum Flavor { Vanilla }"),
        segment("impl Cookie { fn id(&self) -> u8 { 1 } }"),
        segment("struct Cookie;"),
    ];
    let config = NormalizeConfig {
        order: vec![
            ItemOrder::Attributes,
            ItemOrder::Imports,
            ItemOrder::Mods,
            ItemOrder::Macros,
            ItemOrder::Constants,
            ItemOrder::Types,
            ItemOrder::Structs,
            ItemOrder::Impls,
            ItemOrder::Enums,
            ItemOrder::Traits,
            ItemOrder::Foreign,
            ItemOrder::Functions,
            ItemOrder::Tests,
        ],
        ..NormalizeConfig::default()
    };

    let reordered = reorder_items(items, &config, &selection_all());
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &config,
        None,
    );

    let struct_pos = rendered.find("struct Cookie;").expect("struct present");
    let impl_pos = rendered.find("impl Cookie").expect("impl present");
    let enum_pos = rendered.find("enum Flavor").expect("enum present");
    assert!(struct_pos < impl_pos && impl_pos < enum_pos);
}

#[test]
fn compacts_consecutive_mod_items_by_default() {
    let normalized = NormalizedFile {
        shebang: None,
        attrs: Vec::new(),
        items: vec![
            ItemSegment {
                item: parse_item("mod attacks;"),
                leading_comments: Vec::new(),
                module_doc_comments: Vec::new(),
                source: "mod attacks;".to_owned(),
            },
            ItemSegment {
                item: parse_item("mod magics;"),
                leading_comments: Vec::new(),
                module_doc_comments: Vec::new(),
                source: "mod magics;".to_owned(),
            },
            ItemSegment {
                item: parse_item("mod maps;"),
                leading_comments: Vec::new(),
                module_doc_comments: Vec::new(),
                source: "mod maps;".to_owned(),
            },
        ],
    };
    let rendered = render_segments(normalized, &NormalizeConfig::default(), None);
    assert_eq!(rendered, "mod attacks;\nmod magics;\nmod maps;\n");
}

#[test]
fn can_disable_compact_mod_block_via_config() {
    let normalized = NormalizedFile {
        shebang: None,
        attrs: Vec::new(),
        items: vec![
            ItemSegment {
                item: parse_item("mod attacks;"),
                leading_comments: Vec::new(),
                module_doc_comments: Vec::new(),
                source: "mod attacks;".to_owned(),
            },
            ItemSegment {
                item: parse_item("mod magics;"),
                leading_comments: Vec::new(),
                module_doc_comments: Vec::new(),
                source: "mod magics;".to_owned(),
            },
        ],
    };
    let config = NormalizeConfig {
        compact_mod_block: false,
        ..NormalizeConfig::default()
    };
    let rendered = render_segments(normalized, &config, None);
    assert_eq!(rendered, "mod attacks;\n\nmod magics;\n");
}

#[test]
fn treats_multiline_fold_and_trailing_comma_as_whitespace_only() {
    let before = r#"
pub fn sliding_attacks(square: u8, occupancies: u64, directions: &[i8]) -> u64 {
    directions.iter().fold(0, |output, &direction| output | generate_slide(square, occupancies, direction))
}
"#;
    let after = r#"
pub fn sliding_attacks(square: u8, occupancies: u64, directions: &[i8]) -> u64 {
    directions
        .iter()
        .fold(
            0,
            |output, &direction| output | generate_slide(square, occupancies, direction),
        )
}
"#;
    assert!(differs_only_by_whitespace(before, after));
}

#[test]
fn detects_real_non_whitespace_change() {
    let before = "fn f() { let x = 1 + 2; }";
    let after = "fn f() { let x = 1 - 2; }";
    assert!(!differs_only_by_whitespace(before, after));
}

#[test]
fn preserves_plain_comment_before_impl_method() {
    let src = r#"
impl Position {
    pub fn halfmove_clock_bucket(&self) -> usize {
        (self.halfmove_clock().saturating_sub(8) as usize / 8).min(15)
    }

    pub fn hash(&self) -> u64 {
        // To mitigate Graph History Interaction (GHI) problems, the hash key is changed
        // every 8 plies to distinguish between positions that would otherwise appear
        // identical to the transposition table.
        self.state.key ^ ZOBRIST.halfmove_clock[self.halfmove_clock_bucket()]
    }

    pub const fn pawn_key(&self) -> u64 {
        self.state.pawn_key
    }
}
"#;

    let promoted = promote_leading_item_comments(src);
    let parsed = syn::parse_file(&promoted).expect("valid rust file");
    let rendered = render_segments(
        Normalizer::new(parsed, &promoted).normalize(&NormalizeConfig::default(), &selection_all()),
        &NormalizeConfig::default(),
        None,
    );
    let restored = normalize_function_spacing(&restore_promoted_comment_style(&rendered));

    assert!(
        restored.contains(
            "// To mitigate Graph History Interaction (GHI) problems, the hash key is changed"
        ),
        "restored source should keep the method comment"
    );
    assert!(
        restored.contains(
            "// every 8 plies to distinguish between positions that would otherwise appear"
        ),
        "restored source should keep multi-line comment blocks"
    );
}

#[test]
fn keeps_crate_attributes_on_separate_lines() {
    let src = "#![allow(dead_code)]\n#![allow(unused_mut)]\n#![allow(unused_imports)]\n";
    let parsed = syn::parse_file(src).expect("valid rust file");
    let rendered = render_segments(
        Normalizer::new(parsed, src).normalize(&NormalizeConfig::default(), &selection_all()),
        &NormalizeConfig::default(),
        None,
    );
    assert!(!rendered.contains("] #!["));
    assert!(
        rendered.contains("#![allow(dead_code)]\n#![allow(unused_mut)]\n#![allow(unused_imports)]")
    );
}

#[test]
fn can_apply_only_mods_macros_feature() {
    let items = vec![
        segment("fn helper() {}"),
        segment("macro_rules! m { () => {}; }"),
        segment("mod attacks;"),
    ];
    let selection = MoveSelection {
        all: false,
        features: vec![MoveFeature::Mods],
    };
    let reordered = reorder_items(items, &NormalizeConfig::default(), &selection);
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &NormalizeConfig::default(),
        None,
    );

    assert_eq!(
        rendered,
        "fn helper() {}\n\nmod attacks;\n\nmacro_rules! m { () => {}; }\n"
    );
}

#[test]
fn empty_selection_keeps_item_order() {
    let items = vec![
        segment("macro_rules! m { () => {}; }"),
        segment("mod attacks;"),
        segment("fn helper() {}"),
    ];
    let selection = MoveSelection {
        all: false,
        features: Vec::new(),
    };
    let reordered = reorder_items(items, &NormalizeConfig::default(), &selection);
    let rendered = render_segments(
        NormalizedFile {
            shebang: None,
            attrs: Vec::new(),
            items: reordered,
        },
        &NormalizeConfig::default(),
        None,
    );

    assert_eq!(
        rendered,
        "macro_rules! m { () => {}; }\n\nmod attacks;\n\nfn helper() {}\n"
    );
}
