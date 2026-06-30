//! Cross-crate interop with `kira-spatial-field::Field`. Always compiled
//! because the dev-dependency is unconditional; the production path is
//! gated behind the `with-field` feature.

#[cfg(feature = "with-field")]
use kira_spatial_3d::ScalarField;
use kira_spatial_3d::{OwnedScalarField, SpatialDomain};

#[test]
fn owned_scalar_field_round_trips_through_view() {
    let domain = SpatialDomain::new(3, 2, 0.0, 0.0, 1.0, 1.0).unwrap();
    let values = vec![1.0_f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    let owned = OwnedScalarField::new(domain, values.clone()).unwrap();
    let view = owned.as_view();
    assert_eq!(view.values, values.as_slice());
    let (d2, v2) = owned.into_parts();
    assert_eq!(d2, domain);
    assert_eq!(v2, values);
}

#[test]
fn owned_scalar_field_rejects_length_mismatch() {
    let domain = SpatialDomain::new(3, 3, 0.0, 0.0, 1.0, 1.0).unwrap();
    let r = OwnedScalarField::new(domain, vec![1.0_f32; 8]);
    assert!(r.is_err());
}

#[cfg(feature = "with-field")]
#[test]
fn kira_spatial_field_interop_yields_matching_values() {
    use kira_spatial_3d::from_kira_field;
    use kira_spatial_field::{Field, FieldMetadata, NormalizationFlags, ReductionMethod};

    let domain = SpatialDomain::new(2, 2, 0.0, 0.0, 1.0, 1.0).unwrap();
    let values = vec![10.0_f32, 20.0, 30.0, 40.0];
    let metadata = FieldMetadata::builder(
        "test_field".to_string(),
        vec!["G".to_string()],
        ReductionMethod::SingleGene,
    )
    .with_normalization_flags(NormalizationFlags::default())
    .build();
    let f = Field::from_values(1, values.clone(), metadata).unwrap();

    let view: ScalarField<'_> = from_kira_field(domain, &f).unwrap();
    assert_eq!(view.values, values.as_slice());
    assert_eq!(view.domain, domain);
}

#[cfg(feature = "with-field")]
#[test]
fn kira_spatial_field_interop_rejects_length_mismatch() {
    use kira_spatial_3d::from_kira_field;
    use kira_spatial_field::{Field, FieldMetadata, ReductionMethod};

    // 3×3 domain expects 9 values, but the Field has only 4.
    let domain = SpatialDomain::new(3, 3, 0.0, 0.0, 1.0, 1.0).unwrap();
    let metadata = FieldMetadata::builder(
        "mismatch".to_string(),
        vec!["G".to_string()],
        ReductionMethod::SingleGene,
    )
    .build();
    let f = Field::from_values(1, vec![1.0_f32, 2.0, 3.0, 4.0], metadata).unwrap();

    assert!(from_kira_field(domain, &f).is_err());
}
