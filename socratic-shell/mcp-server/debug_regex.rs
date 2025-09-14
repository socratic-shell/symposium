fn main() {
    let text = "Check [file with spaces](src/auth.rs?fn foo) and [file with bracket](src/auth.rs?fn{bar).
Also [main.rs][] and [utils.ts:42][].";
    
    let combined_regex = regex::Regex::new(
        r"(?P<malformed>\[(?P<malformed_text>[^\]]+)\]\((?P<malformed_url>[^)]*[ \{\[\(][^)]*)\))|
          (?P<reference>\[(?P<reference_text>[^\]]+)\]\[\])"
    ).unwrap();
    
    for m in combined_regex.find_iter(text) {
        println!("Match: {:?}", &text[m.start()..m.end()]);
        if let Some(caps) = combined_regex.captures(&text[m.start()..m.end()]) {
            if caps.name("malformed").is_some() {
                println!("  Malformed: text={:?}, url={:?}", 
                    caps.name("malformed_text").unwrap().as_str(),
                    caps.name("malformed_url").unwrap().as_str());
            } else if caps.name("reference").is_some() {
                println!("  Reference: text={:?}", 
                    caps.name("reference_text").unwrap().as_str());
            }
        }
    }
}
