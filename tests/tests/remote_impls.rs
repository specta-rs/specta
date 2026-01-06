#[test]
fn typescript_types_glam() {
    insta::assert_snapshot!(crate::ts::inline::<glam::DVec2>(&Default::default()).unwrap(), @"[number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::IVec2>(&Default::default()).unwrap(), @"[number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::DMat2>(&Default::default()).unwrap(), @"[number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::DAffine2>(&Default::default()).unwrap(), @"[number, number, number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Vec2>(&Default::default()).unwrap(), @"[number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Vec3>(&Default::default()).unwrap(), @"[number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Vec3A>(&Default::default()).unwrap(), @"[number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Vec4>(&Default::default()).unwrap(), @"[number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Mat2>(&Default::default()).unwrap(), @"[number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Mat3>(&Default::default()).unwrap(), @"[number, number, number, number, number, number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Mat3A>(&Default::default()).unwrap(), @"[number, number, number, number, number, number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Mat4>(&Default::default()).unwrap(), @"[number, number, number, number, number, number, number, number, number, number, number, number, number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Quat>(&Default::default()).unwrap(), @"[number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Affine2>(&Default::default()).unwrap(), @"[number, number, number, number, number, number]");
    insta::assert_snapshot!(crate::ts::inline::<glam::Affine3A>(&Default::default()).unwrap(), @"[number, number, number, number, number, number, number, number, number, number, number, number]");
}

#[test]
#[cfg(feature = "bevy_ecs")]
// TODO: This feature guard is bogus because this test package doesn't define any features.
fn typescript_types_bevy_ecs() {
    use specta_typescript::{self, BigIntExportBehavior, ExportConfig, ExportPath};

    assert_eq!(
        ts::inline::<bevy_ecs::entity::Entity>(
            &ExportConfig::default().bigint(BigIntExportBehavior::Number)
        ),
        Ok("number".into())
    );
    // TODO: As we inline `Entity` never ends up in the type map so it falls back to "Entity" in the error instead of the path to the type. Is this what we want or not?
    insta::assert_snapshot!(format!("{:?}", crate::ts::inline::<bevy_ecs::entity::Entity>(&Default::default()).unwrap_err()), @"BigIntForbidden(ExportPath { path: \"Entity -> u64\" })");

    // https://github.com/oscartbeaumont/specta/issues/161#issuecomment-1822735951
    assert_eq!(
        ts::export::<bevy_ecs::entity::Entity>(
            &ExportConfig::default().bigint(BigIntExportBehavior::Number)
        ),
        Ok("export type Entity = number".into())
    );
}
