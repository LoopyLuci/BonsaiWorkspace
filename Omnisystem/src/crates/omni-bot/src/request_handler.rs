use crate::{Result, OmniBotError};

pub struct RequestHandler;

impl RequestHandler {
    pub fn validate_request(command: &str) -> Result<()> {
        if command.is_empty() {
            return Err(OmniBotError::InvalidRequest("Empty command".to_string()));
        }
        Ok(())
    }

    pub fn parse_command(command: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = command.split(':').collect();
        if parts.len() != 2 {
            return Err(OmniBotError::InvalidRequest("Invalid format".to_string()));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    pub fn build_response(service: &str, result: &str) -> String {
        format!("{{{\"service\":\"{}\",\"result\":\"{}\"}}", service, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_request() {
        assert!(RequestHandler::validate_request("test").is_ok());
        assert!(RequestHandler::validate_request("").is_err());
    }

    #[test]
    fn test_parse_command() {
        let (service, payload) = RequestHandler::parse_command("iot:activate_device").unwrap();
        assert_eq!(service, "iot");
        assert_eq!(payload, "activate_device");
    }
}
