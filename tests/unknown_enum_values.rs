//! An enumerated XML attribute holding a value the GDTF specification does not
//! define degrades that one element. It never fails the archive: the value is
//! kept verbatim in the enum's `Other` variant and validation reports it
//! against the geometry it sits on.
//!
//! The defined values are the ones DIN SPEC 15800:2022 lists for each
//! attribute; none of these attributes declares a default.

use std::io::{Cursor, Write};

use gdtf::geometry::{
    ComponentType, CrossSectionType, FuseRating, Geometry, Orientation, StructureType, SupportType,
    WiringObjectGeometry,
};
use gdtf::{GdtfFile, ValidationErrorType, ValidationObject};

/// Wraps a `description.xml` body in the smallest zip that `GdtfFile::new`
/// accepts, so a test can drive the real archive entry point.
fn archive(description: &str) -> Cursor<Vec<u8>> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        zip.start_file::<_, ()>("description.xml", Default::default())
            .expect("start description.xml");
        zip.write_all(description.as_bytes())
            .expect("write description.xml");
        zip.finish().expect("finish archive");
    }
    Cursor::new(buf)
}

const POSITION: &str = "{1,0,0,0}{0,1,0,0}{0,0,1,0}{0,0,0,1}";

/// A description whose `Body` geometry holds the caller's geometry elements.
fn description(geometries: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<GDTF DataVersion="1.2">
  <FixtureType Name="T" ShortName="T" LongName="T" Manufacturer="M" Description="" FixtureTypeID="9E8B4A2C-1D3F-4E5A-8B7C-6D5E4F3A2B1C">
    <AttributeDefinitions>
      <FeatureGroups/>
      <Attributes/>
    </AttributeDefinitions>
    <Wheels/>
    <PhysicalDescriptions/>
    <Models/>
    <Geometries>
      <Geometry Name="Body" Position="{POSITION}">
        {geometries}
      </Geometry>
    </Geometries>
    <DMXModes/>
  </FixtureType>
</GDTF>
"#
    )
}

/// A `<WiringObject>` as GDTF Builder writes it, with every electrical
/// attribute present whatever the component type. This matches the wiring
/// objects in the Martin Professional VDO Sceptron 10 archives.
fn wiring_object(
    component_type: &str,
    fuse_rating: &str,
    orientation: &str,
    voltage: &str,
) -> String {
    format!(
        r#"<WiringObject Name="VMX IN" ComponentType="{component_type}" ConnectorType="Martin BBD" CosPhi="0.000000" ElectricalPayLoad="0.000000" FrequencyRangeMax="0.000000" FrequencyRangeMin="0.000000" FuseCurrent="7.500000" FuseRating="{fuse_rating}" MaxPayLoad="360.000000" Orientation="{orientation}" PinCount="3" Position="{POSITION}" SignalLayer="2" SignalType="VMX" Voltage="{voltage}" VoltageRangeMax="0.000000" VoltageRangeMin="0.000000" WireGroup=""/>"#
    )
}

fn structure(structure_type: &str, cross_section_type: &str) -> String {
    format!(
        r#"<Structure Name="Truss" Position="{POSITION}" LinkedGeometry="Body" StructureType="{structure_type}" CrossSectionType="{cross_section_type}" TrussCrossSection="F34" CrossSectionHeight="0.05" CrossSectionWallThickness="0.002"/>"#
    )
}

fn support(support_type: &str) -> String {
    format!(
        r#"<Support Name="Hoist" Position="{POSITION}" SupportType="{support_type}" RopeCrossSection="R" RopeOffset="{{0,0,1}}" CapacityX="0" CapacityY="0" CapacityZ="0" CapacityXX="0" CapacityYY="0" CapacityZZ="0" ResistanceX="0" ResistanceY="0" ResistanceZ="0" ResistanceXX="0" ResistanceYY="0" ResistanceZZ="0"/>"#
    )
}

fn parse(geometries: &str) -> GdtfFile {
    GdtfFile::new(archive(&description(geometries)))
        .unwrap_or_else(|e| panic!("an undefined enum value must not cost the archive: {e}"))
}

fn the_only_child(file: &GdtfFile) -> &Geometry {
    let Geometry::Generic(body) = &file.description.fixture_types[0].geometries[0] else {
        panic!("expected the Body geometry");
    };
    assert_eq!(body.children.len(), 1);
    &body.children[0]
}

fn wiring(file: &GdtfFile) -> &WiringObjectGeometry {
    match the_only_child(file) {
        Geometry::WiringObject(wiring) => wiring,
        other => panic!("expected a wiring object, got {other:?}"),
    }
}

/// The undefined values validation reports, as `(geometry, attribute, value)`.
fn undefined_values(file: &mut GdtfFile) -> Vec<(String, &'static str, String)> {
    file.validate()
        .errors
        .into_iter()
        .filter_map(|e| match e.ty {
            ValidationErrorType::UndefinedValue(attribute, value) => {
                assert_eq!(
                    e.object,
                    ValidationObject::Geometry,
                    "the report must name the geometry the value sits on"
                );
                Some((e.name.expect("the geometry is named"), attribute, value))
            }
            _ => None,
        })
        .collect()
}

/// `VMXInput` and `VMXOutput` are not among the nine ComponentType values DIN
/// SPEC 15800:2022 defines, yet Martin's VDO Sceptron 10 archives carry them
/// on their VMX data connectors.
#[test]
fn an_undefined_component_type_degrades_the_wiring_object_not_the_archive() {
    for component_type in ["VMXInput", "VMXOutput"] {
        let mut file = parse(&wiring_object(component_type, "B", "Left", "0.000000"));

        let wiring = wiring(&file);
        assert_eq!(
            wiring.component_type,
            ComponentType::Other(component_type.to_owned()),
            "the undefined value must be kept verbatim"
        );
        assert_eq!(
            (
                wiring.signal_type.as_str(),
                wiring.pin_count,
                wiring.signal_layer
            ),
            ("VMX", 3, 2),
            "the rest of the wiring object is still read"
        );

        assert_eq!(
            undefined_values(&mut file),
            [(
                "VMX IN".to_owned(),
                "ComponentType",
                component_type.to_owned()
            )],
            "the undefined value must be reported, not swallowed"
        );
    }
}

/// The fallback must not swallow defined values: a defined component type
/// still parses to its own variant, attributes included.
#[test]
fn a_defined_component_type_keeps_its_variant_and_attributes() {
    let mut file = parse(&wiring_object("PowerSource", "B", "Left", "48.000000"));
    assert_eq!(
        wiring(&file).component_type,
        ComponentType::PowerSource {
            max_pay_load: Some(360.0),
            voltage: Some(48.0),
        }
    );
    assert!(undefined_values(&mut file).is_empty());
}

/// A defined component type whose own attributes are malformed is still an
/// error. It must not be recast as `Other` just because the defined variant
/// failed to parse.
#[test]
fn a_defined_component_type_with_a_malformed_attribute_is_not_recast_as_other() {
    let result = GdtfFile::new(archive(&description(&wiring_object(
        "PowerSource",
        "B",
        "Left",
        "not-a-number",
    ))));
    assert!(
        result.is_err(),
        "a malformed Voltage on a PowerSource must stay an error, got {:?}",
        result.map(|f| wiring(&f).component_type.clone())
    );
}

#[test]
fn an_undefined_fuse_rating_degrades_the_fuse_not_the_archive() {
    let mut file = parse(&wiring_object("Fuse", "gG", "Left", "0.000000"));
    assert_eq!(
        wiring(&file).component_type,
        ComponentType::Fuse {
            fuse_current: Some(7.5),
            fuse_rating: Some(FuseRating::Other("gG".to_owned())),
        }
    );
    assert_eq!(
        undefined_values(&mut file),
        [("VMX IN".to_owned(), "FuseRating", "gG".to_owned())]
    );
}

#[test]
fn an_undefined_orientation_degrades_the_wiring_object_not_the_archive() {
    let mut file = parse(&wiring_object("Input", "B", "Front", "0.000000"));
    let wiring = wiring(&file);
    assert_eq!(wiring.orientation, Orientation::Other("Front".to_owned()));
    assert_eq!(wiring.component_type, ComponentType::Input);
    assert_eq!(
        undefined_values(&mut file),
        [("VMX IN".to_owned(), "Orientation", "Front".to_owned())]
    );
}

#[test]
fn an_undefined_structure_type_degrades_the_structure_not_the_archive() {
    let mut file = parse(&structure("Hybrid", "Tube"));
    let Geometry::Structure(structure) = the_only_child(&file) else {
        panic!("expected a structure");
    };
    assert_eq!(
        structure.structure_type,
        StructureType::Other("Hybrid".to_owned())
    );
    assert_eq!(
        structure.cross_section_type,
        CrossSectionType::Tube {
            height: 0.05,
            wall_thickness: 0.002,
        },
        "the defined cross section next to it is still read"
    );
    assert_eq!(
        undefined_values(&mut file),
        [("Truss".to_owned(), "StructureType", "Hybrid".to_owned())]
    );
}

#[test]
fn an_undefined_cross_section_type_degrades_the_structure_not_the_archive() {
    let mut file = parse(&structure("Detail", "Box"));
    let Geometry::Structure(structure) = the_only_child(&file) else {
        panic!("expected a structure");
    };
    assert_eq!(structure.structure_type, StructureType::Detail);
    assert_eq!(
        structure.cross_section_type,
        CrossSectionType::Other("Box".to_owned())
    );
    assert_eq!(
        undefined_values(&mut file),
        [("Truss".to_owned(), "CrossSectionType", "Box".to_owned())]
    );
}

#[test]
fn an_undefined_support_type_degrades_the_support_not_the_archive() {
    let mut file = parse(&support("Chain"));
    let Geometry::Support(support) = the_only_child(&file) else {
        panic!("expected a support");
    };
    assert_eq!(support.support_type, SupportType::Other("Chain".to_owned()));
    assert_eq!(
        undefined_values(&mut file),
        [("Hoist".to_owned(), "SupportType", "Chain".to_owned())]
    );
}

/// The recovery is per-value, not blanket leniency: geometry whose enumerated
/// values are all defined parses to the defined variants and reports nothing.
#[test]
fn defined_values_report_nothing() {
    for geometry in [
        wiring_object("Fuse", "C", "Bottom", "0.000000"),
        structure("CenterLineBased", "TrussFramework"),
        support("Rope"),
    ] {
        let mut file = parse(&geometry);
        assert!(
            undefined_values(&mut file).is_empty(),
            "{geometry} holds only defined values"
        );
    }
}

/// Validation walks into each geometry's own checks. The undefined-value
/// reports above depend on it, and so does the existing pin patch link check.
#[test]
fn validation_runs_the_checks_of_each_geometry_kind() {
    let mut file = parse(&format!(
        r#"<WiringObject Name="DMX IN" ComponentType="Input" ConnectorType="XLR5" Orientation="Left" PinCount="5" Position="{POSITION}" SignalLayer="1" SignalType="DMX512" WireGroup=""><PinPatch ToWiringObject="Missing" FromPin="1" ToPin="1"/></WiringObject>"#
    ));
    let errors = file.validate().errors;
    assert!(
        errors.iter().any(|e| e.object == ValidationObject::PinPatch
            && matches!(
                e.ty,
                ValidationErrorType::LinkNotFound(ValidationObject::Geometry, _)
            )),
        "a pin patch to a missing wiring object must be reported, got {errors:?}"
    );
}

/// An undefined value is written back as the attribute it came from, so a
/// wiring object survives a serialize and parse round trip unchanged.
#[test]
fn an_undefined_value_serializes_back_to_its_attribute() {
    let xml = wiring_object("VMXInput", "B", "Front", "0.000000");
    let parsed: WiringObjectGeometry = quick_xml::de::from_str(&xml).expect("parse");

    let written = quick_xml::se::to_string_with_root("WiringObject", &parsed).expect("serialize");
    assert!(written.contains(r#"ComponentType="VMXInput""#), "{written}");
    assert!(written.contains(r#"Orientation="Front""#), "{written}");

    let reparsed: WiringObjectGeometry = quick_xml::de::from_str(&written).expect("reparse");
    assert_eq!(reparsed, parsed);
}
