#![cfg(feature = "schemars")]

use bilge::prelude::*;
use schemars::{JsonSchema, schema_for};

#[bitsize(17)]
#[derive(JsonSchemaBits)]
struct BitsStruct {
    padding: u1,
    reserved: u1,
    field1: u8,
    padding: u1,
    field2: u5,
    reserved: u1,
}

#[test]
fn schemars_struct() {
    let schema = schema_for!(BitsStruct);
    assert_eq!(schema.get("type").and_then(|value| value.as_str()), Some("object"));
    let object = schema
        .get("properties")
        .and_then(|value| value.as_object())
        .expect("named bitfield should generate object schema");

    assert_eq!(object.len(), 2);
    assert!(object.contains_key("field1"));
    assert!(object.contains_key("field2"));
    assert!(!object.contains_key("padding_i"));
    assert!(!object.contains_key("reserved_i"));

    let required = schema
        .get("required")
        .and_then(|value| value.as_array())
        .expect("named bitfield should require its fields");
    assert_eq!(required.len(), 2);
    assert!(required.iter().any(|value| value.as_str() == Some("field1")));
    assert!(required.iter().any(|value| value.as_str() == Some("field2")));
    assert_eq!(schema.get("additionalProperties").and_then(|value| value.as_bool()), Some(false));
}

#[bitsize(8)]
#[derive(JsonSchemaBits)]
struct BitsStructWithOnlyPadding {
    padding: u8,
}

#[test]
fn schemars_struct_with_only_padding() {
    let schema = schema_for!(BitsStructWithOnlyPadding);
    assert_eq!(schema.get("type").and_then(|value| value.as_str()), Some("object"));
    assert_eq!(schema.get("additionalProperties").and_then(|value| value.as_bool()), Some(false));
    assert_eq!(schema.get("properties"), None);
    assert_eq!(schema.get("required"), None);
}

#[bitsize(13)]
#[derive(JsonSchemaBits)]
struct BitsTupleStruct(u8, u5);

#[test]
fn schemars_tuple_struct() {
    let schema = schema_for!(BitsTupleStruct);
    assert_eq!(schema.get("type").and_then(|value| value.as_str()), Some("array"));

    assert_eq!(schema.get("minItems").and_then(|value| value.as_u64()), Some(2));
    assert_eq!(schema.get("maxItems").and_then(|value| value.as_u64()), Some(2));

    let items = schema
        .get("prefixItems")
        .and_then(|value| value.as_array())
        .expect("tuple bitfield should define tuple items");
    assert_eq!(items.len(), 2);
}

#[bitsize(8)]
#[derive(JsonSchemaBits)]
struct SchemaIdVisible {
    field: u8,
}

#[bitsize(8, hide_value)]
#[derive(JsonSchemaBits)]
struct SchemaIdHidden {
    field: u8,
}

#[test]
fn json_schema_id_without_hide_value_uses_user_module() {
    assert_eq!(SchemaIdVisible::schema_id().as_ref(), concat!(module_path!(), "::SchemaIdVisible"),);
}

// Known issue: `hide_value` wraps the type in `__bilge_*`, so `module_path!()`
// inside `JsonSchemaBits` is not the user's module.
#[test]
#[should_panic(expected = "__bilge_SchemaIdHidden")]
fn json_schema_id_with_hide_value_does_not_use_user_module() {
    assert_eq!(SchemaIdHidden::schema_id().as_ref(), concat!(module_path!(), "::SchemaIdHidden"),);
}
