use regex_macro::regex;

struct Replacer;

impl regex::Replacer for Replacer {
    fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
        dst.push_str(
            emojis::lookup(&caps[1])
                .map(|e| e.as_str())
                .unwrap_or_else(|| &caps[0]),
        );
    }
}

pub fn replace(text: &str) -> String {
    let re = regex!(r":([a-z1238+-][a-z0-9_-]*):");
    re.replace_all(text, Replacer).into_owned()
}

#[test]
fn smoke() {
    let tests = [
        ("launch nothing", "launch nothing"),
        ("launch :rocket: something", "launch 🚀 something"),
        ("? :unknown: emoji", "? :unknown: emoji"),
        ("::very:naughty::", "::very:naughty::"),
        (":maybe:rocket:", ":maybe:rocket:"),
        (":rocket::rocket:", "🚀🚀"),
    ];

    for (i, o) in tests {
        assert_eq!(replace(i), o);
    }
}
