#![cfg(feature = "schemars")]

use bilge::prelude::*;
use schemars::{
    schema::{InstanceType, SingleOrVec},
    schema_for, JsonSchema,
};

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
    let object = schema.schema.object.expect("named bitfield should generate object schema");
    assert_eq!(schema.schema.instance_type, Some(InstanceType::Object.into()));

    assert_eq!(object.properties.len(), 2);
    assert!(object.properties.contains_key("field1"));
    assert!(object.properties.contains_key("field2"));
    assert!(!object.properties.contains_key("padding_i"));
    assert!(!object.properties.contains_key("reserved_i"));

    assert_eq!(object.required.len(), 2);
    assert!(object.required.contains("field1"));
    assert!(object.required.contains("field2"));
    assert_eq!(object.additional_properties, Some(Box::new(false.into())));
}

#[bitsize(13)]
#[derive(JsonSchemaBits)]
struct BitsTupleStruct(u8, u5);

#[test]
fn schemars_tuple_struct() {
    let schema = schema_for!(BitsTupleStruct);
    let array = schema.schema.array.expect("tuple bitfield should generate array schema");
    assert_eq!(schema.schema.instance_type, Some(InstanceType::Array.into()));

    assert_eq!(array.min_items, Some(2));
    assert_eq!(array.max_items, Some(2));

    let items = array.items.expect("tuple bitfield should define tuple items");
    match items {
        SingleOrVec::Single(_) => panic!("tuple bitfield should have one schema per element"),
        SingleOrVec::Vec(items) => assert_eq!(items.len(), 2),
    }
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
    assert_eq!(
        SchemaIdVisible::schema_id().as_ref(),
        concat!(module_path!(), "::SchemaIdVisible"),
    );
}

// Known issue: `hide_value` wraps the type in `__bilge_*`, so `module_path!()`
// inside `JsonSchemaBits` is not the user's module.
#[test]
#[should_panic(expected = "__bilge_SchemaIdHidden")]
fn json_schema_id_with_hide_value_does_not_use_user_module() {
    assert_eq!(
        SchemaIdHidden::schema_id().as_ref(),
        concat!(module_path!(), "::SchemaIdHidden"),
    );
}
