use datamodel::load_from_file;

#[test]
fn test_load_from_file() {
    let test_file_path = "tests/data/test-simple-v1-1.dmx";

    let result = load_from_file(test_file_path).unwrap();

    let model = result.get_attribute("model").unwrap();

    println!("{:#?}", model)
}
