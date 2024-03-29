// Simplified code from https://github.com/whatisinternet/Inflector

/// Convert string to camelCase
pub fn to_camel_case(convertable_string: &str) -> String {
    let mut new_word: bool = false;
    let mut first_word: bool = false;
    let mut last_char: char = ' ';
    let mut found_real_char: bool = false;
    let mut result: String = String::with_capacity(convertable_string.len() * 2);
    for character in trim_right(convertable_string).chars() {
        if char_is_seperator(character) && found_real_char {
            new_word = true;
        } else if !found_real_char && is_not_alphanumeric(character) {
            continue;
        } else if character.is_numeric() {
            found_real_char = true;
            new_word = true;
            result.push(character);
        } else if last_char_lower_current_is_upper_or_new_word(new_word, last_char, character) {
            found_real_char = true;
            new_word = false;
            result = append_on_new_word(result, first_word, character);
            first_word = false;
        } else {
            found_real_char = true;
            last_char = character;
            result.push(character.to_ascii_lowercase());
        }
    }
    result
}

fn trim_right(convertable_string: &str) -> &str {
    convertable_string.trim_end_matches(is_not_alphanumeric)
}

fn char_is_seperator(character: char) -> bool {
    is_not_alphanumeric(character)
}

fn is_not_alphanumeric(character: char) -> bool {
    !character.is_alphanumeric()
}

fn last_char_lower_current_is_upper_or_new_word(new_word: bool, last_char: char, character: char) -> bool {
    new_word ||
        ((last_char.is_lowercase() && character.is_uppercase()) &&
         (last_char != ' '))
}

#[inline]
fn append_on_new_word(mut result: String, first_word: bool, character: char) -> String {
    if first_word {
        result.push(character.to_ascii_lowercase());
    } else {
        result.push(character.to_ascii_uppercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::to_camel_case;
    #[test]
    pub fn test_to_case_camel() {
        assert_eq!(to_camel_case("tra_ta_ta"), "traTaTa");
    }
}
