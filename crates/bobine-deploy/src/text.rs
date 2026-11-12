pub fn printable(text: &str) -> String {
    text.chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::printable;

    #[test]
    fn network_text_cannot_drive_the_terminal() {
        assert_eq!(printable("borne\u{1b}[31m\u{202E}"), "borne[31m");
    }
}
