use tree_sitter::Language;

extern "C" {
    fn tree_sitter_axiom() -> Language;
}

pub fn get_language() -> Language {
    unsafe { tree_sitter_axiom() }
}

lazy_static::lazy_static! {
    pub static ref LANGUAGE: Language = get_language();
}
