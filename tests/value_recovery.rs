//! A value whose source text does not match the grammar the GDTF
//! specification defines degrades that one element. It never fails the
//! archive, and the substitution is reported against the element it sits on.
//!
//! Expected values here come from the specification (DIN SPEC 15800:2022) and
//! from the two reference implementations, libMVRgdtf (behind Vectorworks,
//! Vision and GDTF Builder) and pygdtf. They are not a transcript of what this
//! crate happened to do before.

use std::io::{Cursor, Write};

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

/// A description carrying one attribute and one channel set, with both values
/// supplied by the caller so a test can make either one malformed.
fn description(attribute_color: &str, channel_set_dmx_from: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<GDTF DataVersion="1.2">
  <FixtureType Name="T" ShortName="T" LongName="T" Manufacturer="M" Description="" FixtureTypeID="9E8B4A2C-1D3F-4E5A-8B7C-6D5E4F3A2B1C">
    <AttributeDefinitions>
      <FeatureGroups>
        <FeatureGroup Name="Color" Pretty="Color">
          <Feature Name="RGB"/>
        </FeatureGroup>
      </FeatureGroups>
      <Attributes>
        <Attribute Name="ColorMacro1" Pretty="Color Macro1" Feature="Color.RGB" Color="{attribute_color}" PhysicalUnit="None"/>
      </Attributes>
    </AttributeDefinitions>
    <Wheels/>
    <PhysicalDescriptions/>
    <Models/>
    <Geometries/>
    <DMXModes>
      <DMXMode Name="Mode" Geometry="Body">
        <DMXChannels>
          <DMXChannel DMXBreak="1" Offset="1" Highlight="255/1" Geometry="Body">
            <LogicalChannel Attribute="ColorMacro1" Snap="No" Master="None" MibFade="0" DMXChangeTimeLimit="0">
              <ChannelFunction Name="F" Attribute="ColorMacro1" OriginalAttribute="" DMXFrom="0/1" Default="0/1" PhysicalFrom="0" PhysicalTo="1" RealFade="0" RealAcceleration="0">
                <ChannelSet Name="Set" DMXFrom="{channel_set_dmx_from}" WheelSlotIndex="0"/>
              </ChannelFunction>
            </LogicalChannel>
          </DMXChannel>
        </DMXChannels>
        <Relations/>
        <FTMacros/>
      </DMXMode>
    </DMXModes>
  </FixtureType>
</GDTF>
"#
    )
}

/// `-255/4` is not a DMXValue: GDTF types the field `Uint/n` and the published
/// XSD pins the grammar to `None|([0-9]+/[1-9][0-9]*s?)`, which admits no sign.
/// libMVRgdtf records an element-level error and substitutes `0` with a byte
/// count of `1`, then parses on; `0/1` is also the default the specification
/// declares for `ChannelSet@DMXFrom`.
#[test]
fn a_malformed_dmx_from_degrades_the_channel_set_not_the_archive() {
    let mut file = GdtfFile::new(archive(&description("0.3127,0.3290,100", "-255/4")))
        .expect("a malformed DMXFrom must not cost the whole archive");

    let fixture_type = &file.description.fixture_types[0];
    let channel_set = &fixture_type.dmx_modes[0].dmx_channels[0].logical_channels[0]
        .channel_functions[0]
        .channel_sets[0];

    assert_eq!(channel_set.dmx_from.value(), 0, "the spec default is 0/1");
    assert_eq!(
        channel_set.dmx_from.bytes().get(),
        1,
        "the spec default is 0/1"
    );
    assert!(
        channel_set.dmx_from.is_substituted(),
        "the substitution must stay visible on the value"
    );

    let errors = file.validate().errors;
    let reported = errors
        .iter()
        .find(|e| matches!(e.ty, ValidationErrorType::SubstitutedValue("DMXFrom")))
        .expect("the substitution must be reported, not swallowed");
    assert_eq!(
        reported.object,
        ValidationObject::ChannelSet,
        "the error must name the element that was degraded"
    );
    assert_eq!(
        reported.name.as_deref(),
        Some("Set"),
        "the error must name which channel set, so a reviewer can find it"
    );
}

/// `-nan(ind)` is MSVC's spelling of an indefinite NaN, emitted when a builder
/// formats a NaN it computed. Both reference parsers fall back to the white
/// point: libMVRgdtf to a default-constructed `CCieColor` and pygdtf to the
/// literal `0.3127, 0.3290, 100.00`.
#[test]
fn a_malformed_attribute_color_degrades_the_attribute_not_the_archive() {
    let mut file = GdtfFile::new(archive(&description(
        "-nan(ind),-nan(ind),-nan(ind)",
        "0/1",
    )))
    .expect("a malformed Color must not cost the whole archive");

    let attribute = &file.description.fixture_types[0]
        .attribute_definitions
        .attributes[0];
    let color = attribute.color.expect("the attribute keeps a color");

    assert_eq!((color.x, color.y, color.z), (0.3127, 0.3290, 100.0));
    assert!(
        color.is_substituted(),
        "the substitution must stay visible on the value"
    );

    let errors = file.validate().errors;
    let reported = errors
        .iter()
        .find(|e| matches!(e.ty, ValidationErrorType::SubstitutedValue("Color")))
        .expect("the substitution must be reported, not swallowed");
    assert_eq!(reported.object, ValidationObject::Attribute);
    assert_eq!(
        reported.name.as_deref(),
        Some("ColorMacro1"),
        "the error must name which attribute, so a reviewer can find it"
    );
}

/// The recovery must not turn into blanket leniency: an archive whose values
/// all match the grammar reports no substitution at all.
#[test]
fn a_well_formed_archive_reports_no_substitution() {
    let mut file =
        GdtfFile::new(archive(&description("0.64,0.33,21.262754", "128/1"))).expect("parse");

    let fixture_type = &file.description.fixture_types[0];
    assert!(
        !fixture_type.attribute_definitions.attributes[0]
            .color
            .expect("color")
            .is_substituted()
    );
    assert!(
        !fixture_type.dmx_modes[0].dmx_channels[0].logical_channels[0].channel_functions[0]
            .channel_sets[0]
            .dmx_from
            .is_substituted()
    );

    let substitutions: Vec<_> = file
        .validate()
        .errors
        .into_iter()
        .filter(|e| matches!(e.ty, ValidationErrorType::SubstitutedValue(_)))
        .collect();
    assert!(
        substitutions.is_empty(),
        "a well-formed archive must report no substitution, got {substitutions:?}"
    );
}

/// Structural damage is still an error. Recovery replaces a value that does not
/// match its grammar; it does not paper over XML that cannot be read at all.
#[test]
fn recovery_does_not_extend_to_malformed_xml() {
    let broken = "<?xml version=\"1.0\"?><GDTF DataVersion=\"1.2\"><FixtureType";
    assert!(
        GdtfFile::new(archive(broken)).is_err(),
        "unparseable XML must still fail; the recovery is per-value, not blanket leniency"
    );
}
