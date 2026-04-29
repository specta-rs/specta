#![allow(deprecated)]

use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Debug)]
struct ErrorStackRootError;

impl std::fmt::Display for ErrorStackRootError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("error stack root error")
    }
}

impl std::error::Error for ErrorStackRootError {}

#[derive(Type)]
struct LegacyImpls {
    ordered_f32: ordered_float::OrderedFloat<f32>,
    ordered_f64: ordered_float::OrderedFloat<f64>,
    heapless_vec: heapless::Vec<i32, 8>,
    semver: semver::Version,
    smol: smol_str::SmolStr,
    array_vec: arrayvec::ArrayVec<i32, 8>,
    array_string: arrayvec::ArrayString<16>,
    smallvec: smallvec::SmallVec<[i32; 8]>,
    toml_datetime: toml::value::Datetime,
    ulid: ulid::Ulid,
    chrono_naive_datetime: chrono::NaiveDateTime,
    chrono_naive_date: chrono::NaiveDate,
    chrono_naive_time: chrono::NaiveTime,
    chrono_duration: chrono::Duration,
    chrono_date: chrono::Date<chrono::Utc>,
    chrono_datetime: chrono::DateTime<chrono::Utc>,
    chrono_fixed_offset: chrono::FixedOffset,
    chrono_utc: chrono::Utc,
    chrono_local: chrono::Local,
    either: either::Either<i32, String>,
    error_stack_report: error_stack::Report<ErrorStackRootError>,
    error_stack_multi_report: error_stack::Report<[ErrorStackRootError]>,
    glam_affine2: glam::Affine2,
    glam_affine3a: glam::Affine3A,
    glam_mat2: glam::Mat2,
    glam_mat3: glam::Mat3,
    glam_mat3a: glam::Mat3A,
    glam_mat4: glam::Mat4,
    glam_quat: glam::Quat,
    glam_vec2: glam::Vec2,
    glam_vec3: glam::Vec3,
    glam_vec3a: glam::Vec3A,
    glam_vec4: glam::Vec4,
    glam_daffine2: glam::DAffine2,
    glam_daffine3: glam::DAffine3,
    glam_dmat2: glam::DMat2,
    glam_dmat3: glam::DMat3,
    glam_dmat4: glam::DMat4,
    glam_dquat: glam::DQuat,
    glam_dvec2: glam::DVec2,
    glam_dvec3: glam::DVec3,
    glam_dvec4: glam::DVec4,
    glam_i8vec2: glam::I8Vec2,
    glam_i8vec3: glam::I8Vec3,
    glam_i8vec4: glam::I8Vec4,
    glam_u8vec2: glam::U8Vec2,
    glam_u8vec3: glam::U8Vec3,
    glam_u8vec4: glam::U8Vec4,
    glam_i16vec2: glam::I16Vec2,
    glam_i16vec3: glam::I16Vec3,
    glam_i16vec4: glam::I16Vec4,
    glam_u16vec2: glam::U16Vec2,
    glam_u16vec3: glam::U16Vec3,
    glam_u16vec4: glam::U16Vec4,
    glam_ivec2: glam::IVec2,
    glam_ivec3: glam::IVec3,
    glam_ivec4: glam::IVec4,
    glam_uvec2: glam::UVec2,
    glam_uvec3: glam::UVec3,
    glam_uvec4: glam::UVec4,
    glam_bvec2: glam::BVec2,
    glam_bvec3: glam::BVec3,
    glam_bvec4: glam::BVec4,
}

#[derive(Type)]
struct LegacyImplWithBigints {
    serde_json_map: serde_json::Map<String, serde_json::Value>,
    serde_json_value: serde_json::Value,
    serde_json_number: serde_json::Number,
    serde_yaml_mapping: serde_yaml::Mapping,
    serde_yaml_tagged: serde_yaml::value::TaggedValue,
    serde_yaml_value: serde_yaml::Value,
    serde_yaml_number: serde_yaml::Number,

    bson_document: bson::Document,
    bson_value: bson::Bson,
    toml_map: toml::map::Map<String, toml::Value>,
    toml_value: toml::Value,

    glam_i64vec2: glam::I64Vec2,
    glam_i64vec3: glam::I64Vec3,
    glam_i64vec4: glam::I64Vec4,
    glam_u64vec2: glam::U64Vec2,
    glam_u64vec3: glam::U64Vec3,
    glam_u64vec4: glam::U64Vec4,
    glam_usizevec2: glam::USizeVec2,
    glam_usizevec3: glam::USizeVec3,
    glam_usizevec4: glam::USizeVec4,
}

#[test]
fn legacy_impls() {
    insta::assert_snapshot!(
        "legacy_impls",
        Typescript::default()
            .export(
                &Types::default().register::<LegacyImpls>(),
                specta_serde::Format
            )
            .unwrap()
    );
}

#[test]
fn legacy_impl_bigint_errors() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<LegacyImplWithBigints>(),
            specta_serde::Format,
        )
        .expect_err("legacy BigInt impls should fail TypeScript export");

    assert!(
        err.to_string()
            .contains("forbids exporting BigInt-style types")
            || err
                .to_string()
                .contains("Detected multiple types with the same name"),
        "unexpected error: {err}"
    );
}

#[test]
fn legacy_impl_individual_bigint_errors() {
    fn assert_bigint_export_error<T: Type>(failures: &mut Vec<String>, name: &str) {
        match Typescript::default().export(&Types::default().register::<T>(), specta_serde::Format)
        {
            Ok(output) => failures.push(format!(
                "{name}: expected BigInt export error, but export succeeded with '{output}'"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("forbids exporting BigInt-style types") => {}
            Err(err) => failures.push(format!("{name}: unexpected error '{err}'")),
        }
    }

    fn assert_bigint_or_invalid_map_key_error<T: Type>(failures: &mut Vec<String>, name: &str) {
        match Typescript::default().export(&Types::default().register::<T>(), specta_serde::Format)
        {
            Ok(output) => failures.push(format!(
                "{name}: expected BigInt or invalid map key error, but export succeeded with '{output}'"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("forbids exporting BigInt-style types") => {}
            Err(err)
                if err
                    .to_string()
                    .contains("Invalid map key") => {}
            Err(err) => failures.push(format!("{name}: unexpected error '{err}'")),
        }
    }

    macro_rules! bigint_wrapper {
        ($name:ident, $ty:ty) => {
            #[derive(Type)]
            #[specta(collect = false)]
            struct $name {
                value: $ty,
            }
        };
    }

    bigint_wrapper!(BsonDocumentBigint, bson::Document);
    bigint_wrapper!(BsonValueBigint, bson::Bson);
    bigint_wrapper!(SerdeJsonMapBigint, serde_json::Map<String, serde_json::Value>);
    bigint_wrapper!(SerdeJsonValueBigint, serde_json::Value);
    bigint_wrapper!(SerdeJsonNumberBigint, serde_json::Number);
    bigint_wrapper!(SerdeYamlMappingBigint, serde_yaml::Mapping);
    bigint_wrapper!(SerdeYamlTaggedBigint, serde_yaml::value::TaggedValue);
    bigint_wrapper!(SerdeYamlValueBigint, serde_yaml::Value);
    bigint_wrapper!(SerdeYamlNumberBigint, serde_yaml::Number);
    bigint_wrapper!(TomlMapBigint, toml::map::Map<String, toml::Value>);
    bigint_wrapper!(TomlValueBigint, toml::Value);
    bigint_wrapper!(GlamI64Vec2Bigint, glam::I64Vec2);
    bigint_wrapper!(GlamI64Vec3Bigint, glam::I64Vec3);
    bigint_wrapper!(GlamI64Vec4Bigint, glam::I64Vec4);
    bigint_wrapper!(GlamU64Vec2Bigint, glam::U64Vec2);
    bigint_wrapper!(GlamU64Vec3Bigint, glam::U64Vec3);
    bigint_wrapper!(GlamU64Vec4Bigint, glam::U64Vec4);
    bigint_wrapper!(GlamUSizeVec2Bigint, glam::USizeVec2);
    bigint_wrapper!(GlamUSizeVec3Bigint, glam::USizeVec3);
    bigint_wrapper!(GlamUSizeVec4Bigint, glam::USizeVec4);

    let mut failures = Vec::new();

    for (name, assert) in [
        (
            "bson::Document",
            assert_bigint_export_error::<BsonDocumentBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "bson::Bson",
            assert_bigint_export_error::<BsonValueBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_json::Map<String, serde_json::Value>",
            assert_bigint_export_error::<SerdeJsonMapBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_json::Value",
            assert_bigint_export_error::<SerdeJsonValueBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_json::Number",
            assert_bigint_export_error::<SerdeJsonNumberBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_yaml::Mapping",
            assert_bigint_or_invalid_map_key_error::<SerdeYamlMappingBigint>
                as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_yaml::value::TaggedValue",
            assert_bigint_or_invalid_map_key_error::<SerdeYamlTaggedBigint>
                as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_yaml::Value",
            assert_bigint_or_invalid_map_key_error::<SerdeYamlValueBigint>
                as fn(&mut Vec<String>, &str),
        ),
        (
            "serde_yaml::Number",
            assert_bigint_export_error::<SerdeYamlNumberBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "toml::map::Map<String, toml::Value>",
            assert_bigint_export_error::<TomlMapBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "toml::Value",
            assert_bigint_export_error::<TomlValueBigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::I64Vec2",
            assert_bigint_export_error::<GlamI64Vec2Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::I64Vec3",
            assert_bigint_export_error::<GlamI64Vec3Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::I64Vec4",
            assert_bigint_export_error::<GlamI64Vec4Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::U64Vec2",
            assert_bigint_export_error::<GlamU64Vec2Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::U64Vec3",
            assert_bigint_export_error::<GlamU64Vec3Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::U64Vec4",
            assert_bigint_export_error::<GlamU64Vec4Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::USizeVec2",
            assert_bigint_export_error::<GlamUSizeVec2Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::USizeVec3",
            assert_bigint_export_error::<GlamUSizeVec3Bigint> as fn(&mut Vec<String>, &str),
        ),
        (
            "glam::USizeVec4",
            assert_bigint_export_error::<GlamUSizeVec4Bigint> as fn(&mut Vec<String>, &str),
        ),
    ] {
        assert(&mut failures, name);
    }

    assert!(
        failures.is_empty(),
        "Unexpected legacy impl BigInt export behavior:\n{}",
        failures.join("\n")
    );
}
