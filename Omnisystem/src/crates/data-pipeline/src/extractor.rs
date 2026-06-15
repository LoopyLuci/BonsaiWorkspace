use crate::Result;

pub struct Extractor;

impl Extractor {
    pub fn extract_from_csv(data: &str) -> Result<Vec<Vec<String>>> {
        let rows: Vec<Vec<String>> = data
            .lines()
            .map(|line| line.split(',').map(|s| s.to_string()).collect())
            .collect();
        Ok(rows)
    }

    pub fn extract_from_json(data: &str) -> Result<Vec<serde_json::Value>> {
        let values: Vec<serde_json::Value> = data
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_extraction() {
        let csv = "a,b,c\n1,2,3";
        let result = Extractor::extract_from_csv(csv).unwrap();
        assert_eq!(result.len(), 2);
    }
}
