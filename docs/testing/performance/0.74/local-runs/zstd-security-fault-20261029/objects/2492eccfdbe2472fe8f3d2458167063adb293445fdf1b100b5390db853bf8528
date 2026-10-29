//! Bounded source/dependency regression only; no product performance measurement.
#[cfg(test)]
mod tests {
    use pulldown_cmark::{html, Options, Parser};

    include!("corpus.rs");

    fn hex(raw: &[u8]) -> String {
        raw.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn empty_display_wikilinks_have_linear_event_growth() {
        let mut observations = Vec::new();
        for repeats in [1usize, 2, 4, 8, 16, 32, 64] {
            let source = "[[a|]] ".repeat(repeats);
            for (label, options) in [
                ("wiki", Options::ENABLE_WIKILINKS),
                (
                    "consumer",
                    Options::all() - Options::ENABLE_SMART_PUNCTUATION,
                ),
            ] {
                // Count without collecting events. The old parser cannot grow process memory
                // exponentially, and the fixed predeclared cap bounds all baseline work.
                let count = Parser::new_ext(&source, options).take(1_000_000).count();
                println!(
                    "HC074-MD-GROWTH option={label} repeats={repeats} count={count} cap=1000000"
                );
                observations.push((repeats, count));
            }
        }
        for (repeats, count) in observations {
            assert_eq!(count, 4 * repeats + 1, "fixed-parser linear event oracle");
        }
    }

    #[test]
    fn consumer_options_complete_corpus_snapshot() {
        for (position, (name, source)) in CORPUS.iter().enumerate() {
            let options = Options::all() - Options::ENABLE_SMART_PUNCTUATION;
            let events: Vec<_> = Parser::new_ext(source, options).collect();
            let mut rendered = String::new();
            html::push_html(&mut rendered, Parser::new_ext(source, options));
            println!(
                "HC074-MD-CORPUS index={position} name={} events={} html={}",
                hex(name.as_bytes()),
                hex(format!("{events:?}").as_bytes()),
                hex(rendered.as_bytes())
            );
        }
        println!("HC074-MD-CORPUS-END count={}", CORPUS.len());
    }

    #[test]
    fn malformed_unicode_and_boundary_inputs_remain_bounded() {
        for source in [
            "",
            "[",
            "[[",
            "[[a|",
            "[[a|]]",
            "[[a|]] [[b|]]",
            "[x](",
            "`",
            "```",
            "é **текст** 🦀",
            "a\0b",
            "\r\n",
            "|a|b|\n|-|-|\n|é|🦀|",
        ] {
            for options in [Options::empty(), Options::all()] {
                let count = Parser::new_ext(source, options).take(4096).count();
                assert!(count < 4096, "bounded short malformed input");
            }
        }
    }
}
