use datamodel::deserialize;

#[test]
fn load_version1_binary() {
    let test_file_path = "tests/data/TestV1BinaryModel.dmx";

    let (_root, header) = deserialize(test_file_path).unwrap();
    assert_eq!(header.encoding_string(), "binary");
    assert_eq!(header.encoding_version(), 1, "Expected encoding version 1, got {}", header.encoding_version());
}

#[test]
fn load_version2_binary() {
    let test_file_path = "tests/data/TestV2BinaryModel.dmx";

    let (_root, header) = deserialize(test_file_path).unwrap();
    assert_eq!(header.encoding_string(), "binary");
    assert_eq!(header.encoding_version(), 2, "Expected encoding version 2, got {}", header.encoding_version());
}

#[test]
fn load_version3_binary() {
    let test_file_path = "tests/data/TestV3BinaryModel.dmx";

    let (_root, header) = deserialize(test_file_path).unwrap();
    assert_eq!(header.encoding_string(), "binary");
    assert_eq!(header.encoding_version(), 3, "Expected encoding version 3, got {}", header.encoding_version());
}

#[test]
fn load_version4_binary() {
    let test_file_path = "tests/data/TestV4BinaryModel.dmx";

    let (_root, header) = deserialize(test_file_path).unwrap();
    assert_eq!(header.encoding_string(), "binary");
    assert_eq!(header.encoding_version(), 4, "Expected encoding version 4, got {}", header.encoding_version());
}

#[test]
fn load_version5_binary() {
    let test_file_path = "tests/data/TestV5BinaryModel.dmx";

    let (_root, header) = deserialize(test_file_path).unwrap();
    assert_eq!(header.encoding_string(), "binary");
    assert_eq!(header.encoding_version(), 5, "Expected encoding version 5, got {}", header.encoding_version());
}
