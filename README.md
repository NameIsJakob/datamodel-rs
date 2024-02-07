# datamodel-rs

datamodel-rs is a Rust library that provides functionality to serialize and deserialize Valve’s proprietary DMX file format.

# What is DMX?

DMX is a file format created by Valve Corporation to store data in a key-value format. It is primarily used for Source Filmmaker (SFM), but it can also be used
for other purposes such as 3D models and particles. DMX files can be saved in either text or binary format. DMX is similar to XML in that it uses elements and
attributes to store data.

# Example

```rs
struct CoolData {
    name: String,
    age: i32,
    is_cool: bool,
}
impl Element for CoolData {
    fn to_element(self) -> DmElement {
        let mut element = DmElement::new(self.name, "CoolData".to_string());
        element.set_attribute("age", self.age);
        element.set_attribute("is_cool", self.is_cool);
        element
    }
    fn from_element(value: &DmElement) -> Option<Self> {
        Some(Self {
            name: value.get_name().to_string(),
            age: *value.get_attribute::<i32>("age")?,
            is_cool: *value.get_attribute::<bool>("is_cool")?,
        })
    }
}

let header = DmHeader {
    encoding_name: "binary".to_string(),
    encoding_version: 2,
    format_name: "dmx".to_string(),
    format_version: 1,
};

let mut root = DmElement::empty();
root.set_name("Cool Data");
root.set_element(
    "The Data",
    CoolData {
        name: "bob".to_string(),
        age: 42,
        is_cool: true,
    },
);

let _ = serialize_file("TheCoolFile.dmx", &root, &header);
```

# What is supported?

-   Binary encoding version 1 - 5 supported

# What is needed?

-   keyvalues2 encoding
-   keyvalues2_flat encoding
