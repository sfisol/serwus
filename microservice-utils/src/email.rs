#[derive(Debug)]
pub struct NoAtSymbolError;

pub fn normalize(val: &str) -> Result<String, NoAtSymbolError> {
    if val.is_empty() || !val.contains('@') {
        return Err(NoAtSymbolError);
    }
    let parts: Vec<&str> = val.rsplitn(2, '@').collect();
    let normalized_domain = parts[0].to_lowercase();

    Ok(format!("{}@{}", parts[1], normalized_domain))
}

pub fn get_domain(val: &str) -> Result<String, NoAtSymbolError> {
    if val.is_empty() || !val.contains('@') {
        return Err(NoAtSymbolError);
    }
    let parts: Vec<&str> = val.rsplitn(2, '@').collect();
    Ok(parts[0].to_string())
}

#[cfg(test)]
mod test {
    use super::normalize;

    #[test]
    fn test_normalize() {
        assert_eq!("Abc@example.com".to_string(), normalize("Abc@EXAMPLE.COM").unwrap());
        assert_eq!("aBc@xYz@example.com".to_string(), normalize("aBc@xYz@Example.COM").unwrap());
    }
}
