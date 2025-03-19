# datamodel-rs

datamodel-rs is a Rust library that provides functionality to serialize and deserialize Valve’s proprietary DMX file format.

# What is DMX?

DMX is a file format created by Valve Corporation to store data in a key-value format. It is primarily used for Source Filmmaker (SFM), but it can also be used
for other purposes such as 3D models and particles. DMX files can be saved in either text or binary format. DMX is similar to XML in that it uses elements and
attributes to store data.

# What is supported?

-   Binary encoding version 1 - 5 supported
-   keyvalues2 encoding
-   keyvalues2_flat encoding
