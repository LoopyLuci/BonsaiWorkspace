use crate::Result;

pub struct WordAligner;

impl WordAligner {
    pub fn align(source: &[String], target: &[String]) -> Result<Vec<(usize, usize)>> {
        let mut alignments = Vec::new();
        
        for (i, _src) in source.iter().enumerate() {
            if i < target.len() {
                alignments.push((i, i));
            }
        }
        
        Ok(alignments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment() {
        let source = vec!["hello".to_string(), "world".to_string()];
        let target = vec!["hola".to_string(), "mundo".to_string()];
        let alignments = WordAligner::align(&source, &target).unwrap();
        assert!(!alignments.is_empty());
    }
}
