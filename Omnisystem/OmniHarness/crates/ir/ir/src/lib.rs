pub struct Ir;
impl Ir { pub fn new() -> Self { Self } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_ir() { let _ = Ir::new(); }
}
