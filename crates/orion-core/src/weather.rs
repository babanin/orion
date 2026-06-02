pub fn parse_temperature_tenths(response: &str) -> Option<i16> {
    let key = "\"temperature_2m\":";
    let mut search = response;
    loop {
        let start = search.find(key)? + key.len();
        let value = parse_temperature_value(&search[start..]);
        if value.is_some() {
            return value;
        }
        search = &search[start..];
    }
}

fn parse_temperature_value(input: &str) -> Option<i16> {
    let mut value = 0_i16;
    let mut sign = 1_i16;
    let mut tenths = 0_i16;
    let mut seen_digit = false;
    let mut after_dot = false;
    for byte in input.as_bytes().iter().copied() {
        match byte {
            b' ' | b'\n' | b'\r' | b'\t' if !seen_digit => {}
            b'-' if !seen_digit => sign = -1,
            b'0'..=b'9' => {
                seen_digit = true;
                if after_dot {
                    tenths = (byte - b'0') as i16;
                    break;
                }
                value = value
                    .saturating_mul(10)
                    .saturating_add((byte - b'0') as i16);
            }
            b'.' => after_dot = true,
            _ => {
                break;
            }
        }
    }
    if seen_digit {
        Some(sign * (value * 10 + tenths))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_meteo_temperature() {
        assert_eq!(
            parse_temperature_tenths(r#"{"current":{"temperature_2m":-4.7}}"#),
            Some(-47)
        );
        assert_eq!(
            parse_temperature_tenths(r#"{"current":{"temperature_2m":12}}"#),
            Some(120)
        );
        assert_eq!(
            parse_temperature_tenths(
                r#"{"current_units":{"temperature_2m":"C"},"current":{"time":"2026-06-01T13:00","temperature_2m":18.4}}"#
            ),
            Some(184)
        );
    }

    #[test]
    fn rejects_missing_or_malformed_temperature() {
        assert_eq!(parse_temperature_tenths(r#"{"current":{}}"#), None);
        assert_eq!(parse_temperature_tenths(r#"{"temperature_2m":null}"#), None);
    }
}
