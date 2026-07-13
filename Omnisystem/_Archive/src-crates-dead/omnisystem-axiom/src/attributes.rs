pub fn parse_axiom_attributes(attrs: &[crate::VerifiedProperty]) -> String {
    let mut output = String::new();
    for attr in attrs {
        output.push_str(&format!(
            "#[axiom::prove({})] // {}\n",
            match attr.property_type {
                crate::PropertyType::NoPanic => "no_panic",
                crate::PropertyType::Idempotent => "idempotent",
                crate::PropertyType::RoundTrip => "round_trip",
                _ => "custom",
            },
            attr.description
        ));
    }
    output
}
