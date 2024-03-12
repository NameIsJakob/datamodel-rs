# datamodel-rs

datamodel-rs is a Rust library that provides functionality to serialize and deserialize Valve’s proprietary DMX file format.

# What is DMX?

DMX is a file format created by Valve Corporation to store data in a key-value format. It is primarily used for Source Filmmaker (SFM), but it can also be used
for other purposes such as 3D models and particles. DMX files can be saved in either text or binary format. DMX is similar to XML in that it uses elements and
attributes to store data.

# What is supported?

-   Binary encoding version 1 - 5 supported

# What is needed?

-   keyvalues2 encoding
-   keyvalues2_flat encoding

# Examples

## Creating An DMX File

```rs
use datamodel as dm;

// A dmx file will always start with a root element.
let mut root = dm::Element::default();

root.add_attribute("Funnier Number", 25);
root.add_attribute("The Half", 0.5);
```

## Creating A Element Class

```rs
// Eventually you will just derive this.

#[derive(Clone, Debug)]
struct TheTest {
    name: String,
    age: i32,
    cool: bool,
}

impl TheTest {
    fn new(name: String, age: i32, cool: bool) -> Self {
        Self { name, age, cool }
    }
}

impl From<TheTest> for dm::Element {
    fn from(test: TheTest) -> Self {
        let mut root = dm::Element::new(test.name, "TheTest");
        root.add_attribute("Age", test.age);
        root.add_attribute("Cool", test.cool);

        root
    }
}

impl From<&Element> for TheTest {
    fn from(value: &Element) -> Self {
        let name = value.name.clone();
        let age = value.get_attribute::<i32>("Age").unwrap();
        let cool = value.get_attribute::<bool>("Cool").unwrap();

        Self::new(name, *age, *cool)
    }
}
```

## Using Element Class

```rs
let the_test = let the_test = TheTest::new("The Test".to_string(), 25, true);

root.add_attribute("The Test", the_test);
```

## Reading Attributes

```rs
let (header, root) = dm::deserialize("test.dmx").unwrap();

let funny = root.get_attribute::<i32>("Funnier Number").unwrap();
let half = root.get_attribute::<f32>("The Half").unwrap();
```

## Reading Elements

```rs
// You have to get the uuid first as a attribute can point towards mutiple elements
let the_testing_id = root.get_attribute("The Test").unwrap();
let the_testing: TheTest = root.get_element(the_testing_id).unwrap();
```
