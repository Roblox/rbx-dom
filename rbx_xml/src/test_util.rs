use std::fmt::Debug;

use crate::{core::XmlType, deserializer_core::XmlEventReader, serializer_core::XmlEventWriter};

pub fn test_xml_round_trip<T>(test_value: &T)
where
    T: XmlType + PartialEq + Debug,
{
    let _ = env_logger::try_init();

    let mut buffer = Vec::new();
    let mut writer = XmlEventWriter::from_output(&mut buffer);

    test_value.write_outer_xml("foo", &mut writer).unwrap();

    println!("{}", std::str::from_utf8(&buffer).unwrap());

    let mut reader = XmlEventReader::from_source(buffer.as_slice());
    reader.next().unwrap().unwrap(); // Eat StartDocument event

    let value = T::read_outer_xml(&mut reader).unwrap();

    assert_eq!(&value, test_value);
}

pub fn test_xml_serialize<T>(expected_source: &str, test_value: &T)
where
    T: XmlType + PartialEq + Debug,
{
    let _ = env_logger::try_init();

    let mut buffer = Vec::new();
    let mut writer = XmlEventWriter::from_output(&mut buffer);

    test_value.write_outer_xml("foo", &mut writer).unwrap();

    let mut expected_events = XmlEventReader::from_source(expected_source.as_bytes());
    let mut actual_events = XmlEventReader::from_source(buffer.as_slice());

    let fail = || {
        panic!(
            "Expected XML:\n{}\n\nActual XML:\n{}",
            expected_source,
            std::str::from_utf8(&buffer).unwrap(),
        )
    };

    loop {
        let (expected, actual) = (expected_events.next(), actual_events.next());

        match (expected.is_some(), actual.is_some()) {
            (true, true) => {
                if expected != actual {
                    println!("Expected event: {:#?}", expected);
                    println!("Actual event: {:#?}", actual);

                    fail();
                }
            }
            (true, false) | (false, true) => {
                println!("Event streams were different lengths!");
                fail()
            }
            (false, false) => break,
        }
    }
}

pub fn test_xml_deserialize<T>(source: &str, expected_value: &T)
where
    T: XmlType + PartialEq + Debug,
{
    let _ = env_logger::try_init();

    let mut reader = XmlEventReader::from_source(source.as_bytes());
    reader.next().unwrap().unwrap(); // Eat StartDocument event
    let value = T::read_outer_xml(&mut reader).unwrap();

    assert_eq!(&value, expected_value);
}
