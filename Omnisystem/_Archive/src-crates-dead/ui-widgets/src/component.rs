use serde::{Deserialize, Serialize};

/// UI component type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentType {
    // Input Controls
    Button,
    TextInput,
    Select,
    Checkbox,
    Radio,
    Toggle,
    Slider,

    // Layout
    Container,
    Grid,
    Flex,
    Modal,
    Sidebar,
    Tabs,

    // Display
    Card,
    Badge,
    Alert,
    Progress,
    Tooltip,
    Skeleton,

    // Navigation
    Menu,
    Breadcrumb,
    Pagination,
    TreeView,

    // Charts
    LineChart,
    BarChart,
    PieChart,
    AreaChart,
}

impl ComponentType {
    pub fn as_str(&self) -> &str {
        match self {
            ComponentType::Button => "button",
            ComponentType::TextInput => "text-input",
            ComponentType::Select => "select",
            ComponentType::Checkbox => "checkbox",
            ComponentType::Radio => "radio",
            ComponentType::Toggle => "toggle",
            ComponentType::Slider => "slider",
            ComponentType::Container => "container",
            ComponentType::Grid => "grid",
            ComponentType::Flex => "flex",
            ComponentType::Modal => "modal",
            ComponentType::Sidebar => "sidebar",
            ComponentType::Tabs => "tabs",
            ComponentType::Card => "card",
            ComponentType::Badge => "badge",
            ComponentType::Alert => "alert",
            ComponentType::Progress => "progress",
            ComponentType::Tooltip => "tooltip",
            ComponentType::Skeleton => "skeleton",
            ComponentType::Menu => "menu",
            ComponentType::Breadcrumb => "breadcrumb",
            ComponentType::Pagination => "pagination",
            ComponentType::TreeView => "tree-view",
            ComponentType::LineChart => "line-chart",
            ComponentType::BarChart => "bar-chart",
            ComponentType::PieChart => "pie-chart",
            ComponentType::AreaChart => "area-chart",
        }
    }
}

/// Component state
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ComponentState {
    Idle,
    Hover,
    Active,
    Focused,
    Disabled,
    Loading,
    Error,
}

/// Component variant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentVariant {
    Primary,
    Secondary,
    Success,
    Danger,
    Warning,
    Info,
}

/// Component size
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// Component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub component_type: ComponentType,
    pub variant: Option<ComponentVariant>,
    pub size: Option<ComponentSize>,
    pub disabled: bool,
    pub aria_label: Option<String>,
}

impl Component {
    pub fn new(name: String, component_type: ComponentType) -> Self {
        Component {
            name,
            component_type,
            variant: None,
            size: None,
            disabled: false,
            aria_label: None,
        }
    }

    pub fn with_variant(mut self, variant: ComponentVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    pub fn with_size(mut self, size: ComponentSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_aria_label(mut self, label: String) -> Self {
        self.aria_label = Some(label);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_type_as_str() {
        assert_eq!(ComponentType::Button.as_str(), "button");
        assert_eq!(ComponentType::Modal.as_str(), "modal");
    }

    #[test]
    fn test_component_creation() {
        let component = Component::new("my-button".to_string(), ComponentType::Button);
        assert_eq!(component.component_type, ComponentType::Button);
        assert!(!component.disabled);
    }

    #[test]
    fn test_component_builder() {
        let component = Component::new("my-button".to_string(), ComponentType::Button)
            .with_variant(ComponentVariant::Primary)
            .with_size(ComponentSize::Large);

        assert_eq!(component.variant, Some(ComponentVariant::Primary));
        assert_eq!(component.size, Some(ComponentSize::Large));
    }

    #[test]
    fn test_math() {
        assert_eq!(2 + 2, 4);
    }
}
