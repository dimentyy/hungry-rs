pub(crate) fn snake_case(s: &str) -> String {
    let mut chars = s.chars().peekable();

    let mut s = String::with_capacity(s.len() + 20);

    let mut previous = '_';

    while let Some(c) = chars.next() {
        match c {
            '_' => {
                if previous != '_' {
                    s.push('_');
                }
            }
            c @ '0'..='9' => {
                if previous.is_ascii_alphabetic() {
                    s.push('_');
                }

                s.push(c);
            }
            c @ 'a'..='z' => {
                if previous.is_ascii_digit() {
                    s.push('_');
                }

                s.push(c);
            }
            c @ 'A'..='Z' => {
                if matches!(previous, '0'..='9' | 'a'..='z')
                    || (chars.peek().map_or(false, char::is_ascii_lowercase) && previous.is_ascii_uppercase())
                {
                    s.push('_');
                }

                s.push(c.to_ascii_lowercase());
            }
            _ => panic!(),
        }

        previous = c;
    }

    s.shrink_to_fit();

    s
}
