enum Casing {
    Upper,
    Lower,
    None,
}

pub(crate) fn pascal_case(s: &str) -> String {
    let chars = s.chars();

    let mut s = String::with_capacity(s.len());

    let mut casing = Casing::Upper;

    for c in chars {
        match c {
            '_' => {
                casing = Casing::Upper;

                continue;
            }
            c @ '0'..='9' => {
                casing = Casing::Upper;

                s.push(c);

                continue;
            }
            'A'..='Z' | 'a'..='z' => {},
            _ => panic!()
        }

        s.push(match casing {
            Casing::Upper => c.to_ascii_uppercase(),
            Casing::Lower => c.to_ascii_lowercase(),
            Casing::None => c,
        });

        casing = if c.is_uppercase() {
            Casing::Lower
        } else {
            Casing::None
        };
    }

    s.shrink_to_fit();

    s
}
