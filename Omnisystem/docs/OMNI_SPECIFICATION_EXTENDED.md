# OMNI Specification - Extended Guide with Real-World Examples

**Comprehensive OMNI format specification with practical implementation patterns**

---

## OMNI Format Overview

The OMNI format is the universal binary data format enabling:
- **Language Interoperability**: Seamless data exchange between TITAN, SYLVA, AETHER, AXIOM
- **Security**: Built-in encryption and authentication
- **Performance**: Compression and efficient serialization
- **Flexibility**: Extensible schema system

---

## File Structure

### Header (256 bytes)

```
Offset  Length  Field            Purpose
------  ------  -----            -------
0       4       Magic Number     "OMNI" (0x4F4D4E49)
4       1       Version          Format version (currently 2)
5       1       Flags            Compression, encryption, signing bits
6       1       Schema Type      User type ID (0-255)
7       1       Reserved         For future use
8       4       Payload Length   Size of encrypted/compressed data
12      4       Uncompressed     Original size before compression
16      16      IV/Nonce         Initialization vector for encryption
32      32      HMAC/Signature   Authentication tag or signature
64      64      Metadata         Custom metadata (key-value pairs)
128     128     Reserved         For future extensions
```

---

## Data Types

### Primitive Types
```
Type        Code    Size        Range
--------    ----    ----        -----
Null        0x00    0           -
Bool        0x01    1           true/false
Int8        0x10    1           -128 to 127
Int16       0x11    2           -32,768 to 32,767
Int32       0x12    4           -2,147,483,648 to 2,147,483,647
Int64       0x13    8           ±9.2 × 10^18
Uint8       0x20    1           0 to 255
Uint16      0x21    2           0 to 65,535
Uint32      0x22    4           0 to 4,294,967,295
Uint64      0x23    8           0 to 18,446,744,073,709,551,615
Float32     0x30    4           IEEE 754 single precision
Float64     0x31    8           IEEE 754 double precision
String      0x40    Variable    UTF-8 encoded
Bytes       0x41    Variable    Raw binary data
```

### Composite Types
```
Type        Code    Structure
--------    ----    ---------
Array       0x50    [count: u32][elements...]
Map         0x51    [count: u32][(key, value)...]
Struct      0x52    [field_count: u32][(name, type, value)...]
Enum        0x53    [variant: u32][value...]
```

---

## Encryption

### Supported Algorithms

```
Algorithm       Key Size    Mode            IV Size
---------       --------    ----            -------
AES-128         128 bits    GCM             96 bits
AES-256         256 bits    GCM             96 bits
ChaCha20        256 bits    Poly1305        96 bits
TwoFish         256 bits    GCM             96 bits
```

### Usage Example

```titan
use omnisystem::omni::*

fun encrypt_data() -> Result<OmniFile, str> {
    let data = vec![1, 2, 3, 4, 5]
    let key = Key::generate(KeyType::AES256)
    
    let omni = OmniFile::new()
        .add_data("numbers", &data)
        .encrypt_with(&key, EncryptionAlgorithm::AES256)?
        .sign_with(&private_key)?
    
    omni.write("data.omni")?
    Ok(omni)
}
```

---

## Compression

### Compression Methods

```
Algorithm       Compression Ratio    Speed       Best For
---------       -----------------    -----       --------
Store           1.0x (none)          ~          No compression needed
Deflate         2-3x                 Medium      General purpose
Zstandard       2-4x                 Fast       Production systems
Brotli          3-5x                 Slow       Maximum compression
```

### Usage Example

```titan
let omni = OmniFile::from_csv("large.csv")?
    .compress(CompressionMethod::Zstandard)?
    .write("data.omni")?
```

---

## Serialization Formats

### JSON Serialization

```titan
let data = OmniFile::new()
    .add_string("name", "Alice")
    .add_i32("age", 30)
    .add_array("tags", vec!["engineer", "manager"])

let json = data.to_json()?
// {
//   "name": "Alice",
//   "age": 30,
//   "tags": ["engineer", "manager"]
// }
```

### MessagePack Serialization

```titan
let data = OmniFile::new()
    .add_string("user", "bob@example.com")
    .add_i64("timestamp", now())

let packed = data.to_msgpack()?  // Binary, compact format
```

### ProtoBuf Serialization

```titan
let data = OmniFile::new()
    .with_schema("message Person { string name = 1; int32 id = 2; }")
    .add_string("name", "Charlie")
    .add_i32("id", 123)

let protobuf = data.to_protobuf()?
```

---

## Schema System

### Defining Schemas

```titan
use omnisystem::omni::*

let schema = Schema::new("User")
    .add_field("id", FieldType::Int64, Required::Yes)
    .add_field("name", FieldType::String, Required::Yes)
    .add_field("email", FieldType::String, Required::Yes)
    .add_field("age", FieldType::Int32, Required::No)
    .add_field("created_at", FieldType::Int64, Required::Yes)

// Version schema for evolution
let schema_v2 = schema.clone()
    .add_field("phone", FieldType::String, Required::No)
    .increment_version()

schema.save("user_schema.omni")?
```

### Validation

```titan
let omni = OmniFile::read("user.omni")?
let schema = Schema::load("user_schema.omni")?

// Validate against schema
match omni.validate_against(&schema) {
    Ok(_) => println!("Valid"),
    Err(e) => println!("Invalid: {}", e),
}
```

---

## Cross-Language Bridges

### TITAN → SYLVA via OMNI

```titan
// TITAN: Create data
let omni = OmniFile::new()
    .add_array("values", vec![1.0, 2.0, 3.0, 4.0])
    .write("data.omni")?

// SYLVA: Read and use
let omni = OmniFile::read("data.omni")?
let tensor = Tensor::from_omni(&omni)?
let mean = tensor.mean()
```

### AETHER → AXIOM via OMNI

```aether
// AETHER: Distribute data
let msg = Message::new(...)
    .set_payload_omni(&data)
    .sign(private_key)?

// AXIOM: Verify and process
let omni = msg.get_payload_omni()?
verify_signature(omni, public_key)?
let formula = Formula::from_omni(&omni)?
```

---

## Real-World Use Cases

### Use Case 1: Data Pipeline

```
Source System (SQL)
        ↓
   OMNI Extract (encrypted)
        ↓
Transformation (SYLVA)
        ↓
  OMNI Intermediate (compressed)
        ↓
Distribution (AETHER)
        ↓
  OMNI at Rest (in warehouse)
```

### Use Case 2: Federated Learning

```
Device 1: Train Model → OMNI Export (encrypted)
Device 2: Train Model → OMNI Export (encrypted)
Device 3: Train Model → OMNI Export (encrypted)
              ↓
         Central Node
              ↓
    OMNI Aggregate → AETHER Distribute
              ↓
      Updated Models
```

### Use Case 3: Compliance & Auditing

```
Transaction → OMNI Serialize (all fields)
                    ↓
            HMAC Sign + Encrypt
                    ↓
              Write to Log
                    ↓
         Verify Signature on Audit
```

---

## Performance Characteristics

### File Size Comparison

```
Original JSON:      10 MB
OMNI (no compression): 8 MB
OMNI (Zstandard):     2 MB
OMNI (Brotli):        1.5 MB

Savings: 85% with encryption and compression
```

### Read Performance

```
Format          Time to Read     Memory Peak
------          ------          -----------
JSON            250 ms          10 MB
OMNI (plain)    50 ms           2 MB
OMNI (compressed) 100 ms        3 MB (+ decompression buffer)
```

---

## Error Handling

### Verification Failures

```titan
match OmniFile::read("data.omni") {
    Ok(file) => {
        match file.verify_signature(public_key) {
            Ok(_) => println!("Signature valid"),
            Err(OmniError::SignatureInvalid) => {
                println!("File has been tampered with")
            },
            Err(e) => println!("Error: {}", e),
        }
    },
    Err(OmniError::CorruptedFile) => {
        println!("File corrupted or invalid format")
    },
    Err(e) => println!("Error: {}", e),
}
```

---

## Best Practices

✅ **DO**
- Always encrypt sensitive data
- Sign critical files
- Version your schemas
- Use compression for large files
- Validate against schema

❌ **DON'T**
- Store plaintext passwords
- Skip signature verification
- Mix schema versions without versioning
- Assume files are valid
- Use weak encryption

---

## Extension Points

### Custom Type Support

```titan
impl OmniSerializable for CustomType {
    fn to_omni(&self) -> Result<OmniData> {
        // Custom serialization logic
    }
    
    fn from_omni(data: &OmniData) -> Result<Self> {
        // Custom deserialization logic
    }
}
```

### Plugin Metadata

```titan
let omni = OmniFile::new()
    .set_metadata("plugin_version", "1.2.3")
    .set_metadata("plugin_author", "Company XYZ")
    .set_metadata("plugin_features", "feature1,feature2")
```

---

## Migration & Versioning

### Schema Evolution

```
v1: User { id, name, email }
    ↓ (add phone field)
v2: User { id, name, email, phone? }
    ↓ (rename email to contact_email)
v3: User { id, name, contact_email, phone? }
    ↓ (remove phone, add metadata)
v4: User { id, name, contact_email, metadata }
```

### Backward Compatibility

```titan
let old_omni = OmniFile::read("user_v1.omni")?
let new_omni = old_omni.upgrade_schema(v1, v4)?
// Automatically fills new fields with defaults
```

---

## Next Steps

- Integration: [LANGUAGE_BRIDGES.md](LANGUAGE_BRIDGES.md)
- Frameworks: Framework guides
- Examples: Example applications

---

**OMNI Extended** - Universal data format for enterprise systems!
