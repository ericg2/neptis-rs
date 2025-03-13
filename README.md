# Neptis Rust API
This API is auto-generated using OpenAPI Generator on the `v1-final.json` file.

It will be synchronized with the latest version.

## Building / Generating
To re-generate, use the following command:
```
npm install @openapitools/openapi-generator-cli
git clone https://github.com/ericg2/neptis-rs.git
cd neptis-rs
npx @openapitools/openapi-generator-cli generate \
    -i v1-final.json \
    -g rust \
    -o v1final
```

You will then need to install these dependencies:
```
serde = { version = "XYZ", features = ["derive"] }
serde_repr = "XYZ"
serde_json = "XYZ"
serde_with = "XYZ
url = "XYZ"
reqwest = { version = "XYZ", features = ["json"] }
uuid = { version = "XYZ", features = [..., "serde"] }
```
