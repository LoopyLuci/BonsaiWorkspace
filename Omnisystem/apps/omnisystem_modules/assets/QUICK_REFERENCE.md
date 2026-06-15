# 🎨 Universal Asset Framework - Quick Reference

**Fast lookup guide for common operations and patterns**

---

## 🔧 System Initialization

```titan
// Initialize the entire asset system
let system = omnisystem::assets::initialize_asset_system()?

// Output: 8 initialization steps, ready for use
```

---

## 📦 Asset Types

```titan
enum AssetType {
    WebComponent,      // React/Vue/Angular components
    GameAsset,        // 3D models, textures, animations
    VisualAsset,      // Icons, illustrations, designs
    AudioAsset,       // Music, effects, voices
    DataAsset,        // Data structures
    ModelAsset,       // AI/ML models
    IntegrationAsset, // API integrations
}
```

---

## 🎨 Web Components

### Create a Button Component
```titan
let builder = create_component_builder(
    ComponentType::Button,
    UILibrary::React,
    "ModernButton".to_string(),
)

let component = builder
    .add_prop("color".to_string(), "string".to_string(), true)
    .add_prop("onClick".to_string(), "function".to_string(), false)
    .add_state("isHovered".to_string(), "bool".to_string(), "false".to_string())
    .build_component()?
```

### Convert to Vue
```titan
let vue_component = convert_component(&component, UILibrary::Vue)?
```

### Export as JSX
```titan
let code = export_component(&component, ExportFormat::JSX)?
```

---

## 🎮 Game Assets

### Import 3D Model
```titan
let model = import_model(
    "/path/to/model.fbx".to_string(),
    GameEngine::Unity,
)?

// Optimize and generate LODs
let optimized = optimize_model(model)?
```

### Generate Texture
```titan
let texture = generate_texture(1024, 1024, TextureType::Normal)?

// Convert to different format
let compressed = convert_texture(texture, TextureFormat::DDS)?
```

### Export for Engine
```titan
let export_path = export_for_engine(&model, GameEngine::Unreal)?
// Result: /exports/model_xxx.uasset
```

---

## 🎨 Visual Assets

### Generate Icon Set
```titan
let icons = generate_icon_set(
    "navigation".to_string(),
    StylePreset::Minimalist,
    vec!["home".to_string(), "settings".to_string(), "help".to_string()],
)?
```

### Create Design System
```titan
let design_system = create_design_system(
    "Modern".to_string(),
    "#00D4FF".to_string(),
)?
```

### Export Design System
```titan
let css = export_design_system(&design_system, DesignFormat::CSS)?
let json = export_design_system(&design_system, DesignFormat::JSON)?
```

---

## 🎵 Audio Assets

### Generate Music
```titan
let request = AudioGenerationRequest {
    prompt: "Ambient electronic music".to_string(),
    duration_seconds: 180.0,
    style: AudioStyle::Electronic,
    genre: AudioGenre::Electronic,
    mood: AudioMood::Calm,
    sample_rate: SampleRate::Hz48000,
    bit_depth: BitDepth::Bit24,
    // ... other params
}

let music = generate_music_track(&request)?
```

### Generate Sound Effect
```titan
let sfx = generate_sound_effect(
    "Door opening with creaking".to_string(),
    2.5,  // seconds
)?
```

### Text-to-Speech
```titan
let tts_request = TextToSpeechRequest {
    text: "Welcome to Omnisystem".to_string(),
    voice_id: "voice_01".to_string(),
    emotion: SpeechEmotion::Happy,
    speed: 1.0,
    // ... other params
}

let voice = synthesize_speech(&tts_request)?
```

### Audio Processing
```titan
let processor = create_processor()
    .add_effect(AudioEffect { /* reverb */ })
    .add_effect(AudioEffect { /* compression */ })

let processed = apply_reverb(audio, 0.5, 0.3)?
let normalized = normalize_audio(processed, 0.95)?
```

---

## 📚 Asset Library

### Search Assets
```titan
let query = AssetSearchQuery {
    search_term: Some("button".to_string()),
    asset_type: Some(AssetType::WebComponent),
    category: Some(AssetCategory::Button),
    generation_method: Some(GenerationMethod::Procedural),
    status: Some(AssetStatus::Active),
    min_rating: Some(4.0),
    tags: vec!["accessible".to_string()],
    sort_by: SortField::Rating,
    page: 1,
    limit: 10,
}

let results = search_all_assets(&system, &query)
```

### Find by Type/Category
```titan
let web_components = get_assets_by_type(&library, AssetType::WebComponent)
let buttons = get_assets_by_category(&library, AssetCategory::Button)
```

### Trending & Top Rated
```titan
let trending = get_trending_assets(&library, 10)
let top_rated = get_top_rated_assets(&library, 10)
```

### Get Statistics
```titan
let stats = get_system_statistics(&system)
println!("Total assets: {}", stats.total_assets)
println!("Avg rating: {:.1}", stats.average_rating)
```

---

## 🔄 Asset Generation

### Procedural Generation
```titan
let request = GenerationRequest {
    asset_type: AssetType::WebComponent,
    generation_method: GenerationMethod::Procedural,
    prompt: "Modern card with shadow".to_string(),
    // ...
}

let (system, asset) = generate_asset(system, &request)?
```

### Deterministic Generation (Reproducible)
```titan
let request = GenerationRequest {
    generation_method: GenerationMethod::Deterministic,
    parameters: GenerationParameters {
        seed: Some(12345),  // Same seed = same result
        // ...
    },
    // ...
}

let (system, asset) = generate_asset(system, &request)?
```

### Hybrid Generation
```titan
let request = GenerationRequest {
    generation_method: GenerationMethod::Hybrid,
    // Uses procedural rules + deterministic variation
    // ...
}

let (system, asset) = generate_asset(system, &request)?
```

### AI-Assisted Generation
```titan
let request = GenerationRequest {
    generation_method: GenerationMethod::AIAssisted,
    prompt: "Dashboard with real-time metrics and dark theme".to_string(),
    // Natural language → AI processes → asset created
    // ...
}

let (system, asset) = generate_asset(system, &request)?
```

---

## 📝 Metadata Operations

### Create Asset
```titan
let mut asset = create_asset_metadata(
    "My Button".to_string(),
    "A modern button component".to_string(),
    AssetType::WebComponent,
    AssetCategory::Button,
    "john.doe".to_string(),
)
```

### Update Metadata
```titan
let asset = update_asset_metadata(
    asset,
    Some("Updated description".to_string()),
    Some(vec!["accessible".to_string(), "responsive".to_string()]),
    Some(AssetStatus::Active),
)
```

### Publish Asset
```titan
let published = publish_asset(asset)?  // Draft → Active
```

### Rate Asset
```titan
let rated = update_rating(asset, 4.8)  // Out of 5.0
```

### Track Usage
```titan
let asset = increment_download_count(asset)
let asset = increment_usage_count(asset)
```

---

## 📥 Batch Operations

### Import Multiple Assets
```titan
let system = import_assets_batch(
    system,
    vec![asset1, asset2, asset3],
    "Web Assets".to_string(),
)?
```

### Export Library
```titan
let export_result = export_library(
    &system,
    "Web Assets".to_string(),
    LibraryExportFormat::Archive,
)?
```

---

## 🔍 Search Sorting Options

```titan
enum SortField {
    Name,           // Alphabetical
    DateCreated,    // Newest first
    DateModified,   // Recently updated
    Rating,         // Highest rated first
    Downloads,      // Most downloaded first
    Usage,          // Most used first
    Relevance,      // Search term match quality
}
```

---

## 🎯 Asset Status Lifecycle

```
Draft → Active → Archived
         ↓
      Deprecated
      
InReview (special state)
```

---

## 🏷️ Asset Categories

**Web**: Button, Card, Form, Table, Modal, Navigation, Icon, Other

**Game**: Model3D, Texture, Material, Animation, Sprite, Effect, Sound, PhysicsBody, Prefab

**Visual**: Icon, Illustration, Photography, Diagram, Chart, DesignSystem, EffectComposition, Gradient, Pattern

**Audio**: MusicTrack, SoundEffect, TextToSpeech, Ambient, Narration, Jingle, Foley, Dialogue

---

## 🎨 Style Presets

```
Minimalist    | Modern         | Vintage
CyberPunk     | Flat           | 3D
Sketch        | Watercolor     | Photography
Abstract
```

---

## 📊 Common Patterns

### Get All High-Rated Components
```titan
let high_rated = get_top_rated_assets(&library, 100)
    .iter()
    .filter(|a| a.asset_type == AssetType::WebComponent)
    .collect()
```

### Generate and Publish
```titan
let (system, asset) = generate_asset(system, &request)?
let published = publish_asset(asset)?
let system = add_asset_to_system(system, published, "Web Assets".to_string())?
```

### Search + Filter + Sort
```titan
let results = search_assets(&library, &query)
    .iter()
    .filter(|a| a.rating >= 4.0)
    .collect()
```

---

## 🚨 Error Handling

```titan
match generate_asset(system, &request) {
    Ok((system, asset)) => {
        // Use asset
    },
    Err(e) => {
        println!("Generation failed: {}", e)
        // Handle error
    }
}
```

---

## ⚡ Performance Tips

1. **Search**: Use specific filters to reduce result set
2. **Caching**: Cache library statistics for dashboard use
3. **Pagination**: Always use page/limit for large results
4. **Batch**: Use batch import for multiple assets
5. **Generation**: Seed-based generation is faster than AI-assisted

---

## 📞 Module Imports

```titan
import omnisystem.assets
import omnisystem.assets.asset_metadata
import omnisystem.assets.asset_library
import omnisystem.assets.hybrid_asset_generator
import omnisystem.assets.web_framework
import omnisystem.assets.game_framework
import omnisystem.assets.visual_framework
import omnisystem.assets.audio_framework
```

---

## 🎓 Complete Example

```titan
// 1. Initialize
let system = omnisystem::assets::initialize_asset_system()?

// 2. Generate web component
let web_request = GenerationRequest { /* ... */ }
let (system, web_component) = generate_asset(system, &web_request)?

// 3. Publish to library
let published = publish_asset(web_component)?
let system = add_asset_to_system(
    system,
    published,
    "Web Assets".to_string()
)?

// 4. Search for similar
let query = AssetSearchQuery {
    asset_type: Some(AssetType::WebComponent),
    sort_by: SortField::Rating,
    // ...
}
let similar = search_all_assets(&system, &query)

// 5. Get stats
let stats = get_system_statistics(&system)
println!("Total: {}, Avg Rating: {}", 
    stats.total_assets, 
    stats.average_rating
)
```

---

**For complete details, see README.md and IMPLEMENTATION_SUMMARY.md**
